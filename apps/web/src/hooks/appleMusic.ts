import { useCallback, useEffect, useMemo, useState } from "react"
import {
  usePlaylistContext,
  type ApplePlaylist,
} from "@/providers/playlistsProvider"
import { useSearchProvider } from "@/providers/searchProvider"

// ---------------------------------------------------------------------------
// Minimal ambient MusicKit JS types — just the surface this hook touches.
// No @types/musickit-js is installed; declaring the shape locally avoids
// pulling in a dependency for three method calls.
// ---------------------------------------------------------------------------

type MusicKitInstance = {
  authorize: () => Promise<string>
  musicUserToken: string
  storefrontId: string
}

declare global {
  interface Window {
    MusicKit?: {
      configure: (config: {
        developerToken: string
        app: { name: string; build: string }
      }) => void
      getInstance: () => MusicKitInstance
    }
  }
}

const MUSICKIT_SRC = "https://js-cdn.music.apple.com/musickit/v3/musickit.js"

let musicKitLoadPromise: Promise<void> | null = null

/** Loads the MusicKit JS script once and resolves once it's ready to configure. */
function loadMusicKit(): Promise<void> {
  if (window.MusicKit) return Promise.resolve()
  if (musicKitLoadPromise) return musicKitLoadPromise

  musicKitLoadPromise = new Promise((resolve, reject) => {
    document.addEventListener("musickitloaded", () => resolve(), { once: true })
    const script = document.createElement("script")
    script.src = MUSICKIT_SRC
    script.onerror = () => reject(new Error("Failed to load MusicKit JS"))
    document.head.appendChild(script)
  })

  return musicKitLoadPromise
}

const DEVICE_ID_KEY = "gigeo_apple_device_id"

/**
 * A persistent client-generated UUID standing in for Apple's lack of a
 * stable user ID via MusicKit — see the Phase 0 plan's "identity mapping"
 * decision and `apple_handlers::db` on the backend.
 */
function getOrCreateDeviceId(): string {
  const existing = localStorage.getItem(DEVICE_ID_KEY)
  if (existing) return existing
  const id = crypto.randomUUID()
  localStorage.setItem(DEVICE_ID_KEY, id)
  return id
}

export type AppleAuthStatus = {
  logged_in: boolean
  apple_music_connected: boolean
}

export type CreateApplePlaylistInput = {
  name: string
  location: string
  description: string
  cadence: number
  radius: number
}

export type CreateApplePlaylistOutput = {
  playlist_id: string
  track_count: number
}

export type UpdateApplePlaylistInput = {
  cadence: number
}

type UseAppleMusicAuthResult = {
  status: AppleAuthStatus | null
  loading: boolean
  error: string | null
  refresh: () => Promise<void>
  connectAppleMusic: () => Promise<void>
  logout: () => Promise<void>
  createPlaylist: (input: FormData) => Promise<CreateApplePlaylistOutput>
  getPlaylists: () => Promise<ApplePlaylist[]>
  updatePlaylist: (
    playlistId: string,
    input: UpdateApplePlaylistInput
  ) => Promise<void>
  deletePlaylist: (playlistId: string) => Promise<void>
}

export function parseCreateApplePlaylistForm(
  formData: FormData
): CreateApplePlaylistInput {
  const name = formData.get("playlistName")
  const location = formData.get("location")
  const cadence = formData.get("cadence")

  if (typeof name !== "string" || !name.trim()) {
    throw new Error("Playlist name is required")
  }
  if (typeof location !== "string" || !location.trim()) {
    throw new Error("Location is required")
  }
  if (typeof cadence !== "string" || !cadence) {
    throw new Error("Cadence is required")
  }

  return {
    name: name.trim(),
    location: location.trim(),
    description: "",
    cadence: cadence === "bimonthly" ? 60 : cadence === "weekly" ? 7 : 30,
    radius: 25,
  }
}

export function parseUpdateApplePlaylistForm(
  formData: FormData
): UpdateApplePlaylistInput {
  const cadence = formData.get("cadence")
  if (typeof cadence !== "string" || !cadence) {
    throw new Error("Cadence is required")
  }
  return {
    cadence: cadence === "bimonthly" ? 60 : cadence === "weekly" ? 7 : 30,
  }
}

export function useAppleMusicAuth(): UseAppleMusicAuthResult {
  const { setApplePlaylists, setPlaylistManagement } = usePlaylistContext()
  const { selectedCoordinates } = useSearchProvider()
  const [status, setStatus] = useState<AppleAuthStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const { latitude, longitude } = {
    latitude: selectedCoordinates[1],
    longitude: selectedCoordinates[0],
  }

  const refresh = useCallback(async () => {
    setLoading(true)
    setError(null)

    try {
      const res = await fetch("api/apple/auth-status", {
        credentials: "include",
      })
      if (!res.ok) {
        throw new Error(`status fetch failed: ${res.status}`)
      }
      const data: AppleAuthStatus = await res.json()
      setStatus(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : "unknown error")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void refresh()
  }, [refresh])

  const connectAppleMusic = useCallback(async () => {
    setError(null)
    try {
      await loadMusicKit()
      const tokenRes = await fetch("api/apple/developer-token", {
        credentials: "include",
      })
      if (!tokenRes.ok) {
        throw new Error(`developer token fetch failed: ${tokenRes.status}`)
      }
      const { developer_token } = await tokenRes.json()

      if (!window.MusicKit) {
        throw new Error("MusicKit failed to load")
      }
      window.MusicKit.configure({
        developerToken: developer_token,
        app: { name: "Gigeo", build: "1" },
      })
      const music = window.MusicKit.getInstance()
      const musicUserToken = await music.authorize()

      const connectRes = await fetch("api/apple/connect", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          device_id: getOrCreateDeviceId(),
          music_user_token: musicUserToken,
          storefront: music.storefrontId || "us",
        }),
      })
      if (!connectRes.ok) {
        const text = await connectRes.text()
        throw new Error(text || `connect failed: ${connectRes.status}`)
      }

      await refresh()
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to connect Apple Music"
      )
    }
  }, [refresh])

  const logout = useCallback(async () => {
    setError(null)
    const res = await fetch("api/apple/logout", {
      method: "POST",
      credentials: "include",
    })
    if (!res.ok) {
      throw new Error(`logout failed: ${res.status}`)
    }
    await refresh()
  }, [refresh])

  const getPlaylists = useCallback(async () => {
    setError(null)
    const res: ApplePlaylist[] = await fetch("api/apple/playlist", {
      method: "GET",
      credentials: "include",
    }).then((res) => res.json())
    setApplePlaylists(res)
    setPlaylistManagement("apple")
    return res
  }, [setApplePlaylists, setPlaylistManagement])

  useEffect(() => {
    if (status?.apple_music_connected) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      void getPlaylists()
    }
  }, [status?.apple_music_connected, getPlaylists])

  const createPlaylist = useCallback(
    async (input: FormData) => {
      setError(null)
      const payload = parseCreateApplePlaylistForm(input)

      const res = await fetch("api/apple/playlist", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: payload.name,
          description: payload.description || null,
          location: payload.location,
          cadence: payload.cadence,
          latitude,
          longitude,
        }),
      })

      if (!res.ok) {
        const text = await res.text()
        throw new Error(text || `create playlist failed: ${res.status}`)
      }

      const data: CreateApplePlaylistOutput = await res.json()
      await refresh()
      return data
    },
    [refresh, latitude, longitude]
  )

  const updatePlaylist = useCallback(
    async (playlistId: string, input: UpdateApplePlaylistInput) => {
      setError(null)
      const res = await fetch(`api/apple/playlist/${playlistId}`, {
        method: "PATCH",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      })
      if (!res.ok) {
        const text = await res.text()
        throw new Error(text || `update playlist failed: ${res.status}`)
      }
      await getPlaylists()
    },
    [getPlaylists]
  )

  const deletePlaylist = useCallback(
    async (playlistId: string) => {
      setError(null)
      const res = await fetch(`api/apple/playlist/${playlistId}`, {
        method: "DELETE",
        credentials: "include",
      })
      if (!res.ok) {
        const text = await res.text()
        throw new Error(text || `delete playlist failed: ${res.status}`)
      }
      await getPlaylists()
    },
    [getPlaylists]
  )

  return useMemo(
    () => ({
      status,
      loading,
      error,
      refresh,
      connectAppleMusic,
      logout,
      createPlaylist,
      getPlaylists,
      updatePlaylist,
      deletePlaylist,
    }),
    [
      status,
      loading,
      error,
      refresh,
      connectAppleMusic,
      logout,
      createPlaylist,
      getPlaylists,
      updatePlaylist,
      deletePlaylist,
    ]
  )
}
