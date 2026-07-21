import { useCallback, useEffect, useMemo, useState } from "react"
import { usePlaylistContext } from "@/providers/playlistsProvider"
import { useSearchProvider } from "@/providers/searchProvider"
export type AuthStatus = {
  logged_in: boolean
  spotify_connected: boolean
  spotify_user_id: string | null
  can_create_private_playlist: boolean
  can_create_public_playlist: boolean
}

export type CreatePlaylistInput = {
  name: string
  location: string
  description: string
  privacy: boolean
  cadence: number
  radius: number
}

export type CreatePlaylistOutput = {
  id: string
  name: string
  spotify_url?: string | null
}

type UseSpotifyAuthResult = {
  status: AuthStatus | null
  loading: boolean
  error: string | null
  refresh: () => Promise<void>
  connectSpotify: () => void
  logout: () => Promise<void>
  createPlaylist: (input: FormData) => Promise<CreatePlaylistOutput>
  getPlaylists: () => Promise<any>
}

function parseCreatePlaylistForm(formData: FormData): CreatePlaylistInput {
  const name = formData.get("playlistName")
  const location = formData.get("location")
  const privacy = formData.get("privacy")
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
    description: "test description",
    privacy: privacy === "private",
    cadence: cadence === "daily" ? 1 : cadence === "weekly" ? 7 : 30,
    radius: 25,
  }
}

export function useSpotifyAuth(): UseSpotifyAuthResult {
  const { setSpotifyPlaylists, setPlaylistManagement } = usePlaylistContext()
  const { selectedCoordinates } = useSearchProvider()
  const [status, setStatus] = useState<AuthStatus | null>(null)
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
      const res = await fetch("api/auth/status", {
        credentials: "include",
      })

      if (!res.ok) {
        throw new Error(`status fetch failed: ${res.status}`)
      }

      const data: AuthStatus = await res.json()
      setStatus(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : "unknown error")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const connectSpotify = useCallback(() => {
    window.location.assign("/api/spotify/login")
  }, [])

  const logout = useCallback(async () => {
    setError(null)

    const res = await fetch(`api/auth/logout`, {
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

    const res = await fetch(`api/spotify/playlist`, {
      method: "GET",
      credentials: "include",
    }).then((res) => res.json())
    setSpotifyPlaylists(res)
    setPlaylistManagement("spotify")
    console.log(res)
  }, [refresh])

  const createPlaylist = useCallback(
    async (input: FormData) => {
      setError(null)
      const payload = parseCreatePlaylistForm(input)

      console.log(latitude, longitude, selectedCoordinates)

      const res = await fetch(`api/spotify/playlist`, {
        method: "POST",
        credentials: "include",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          ...payload,
          latitude,
          longitude,
        }),
      })

      if (!res.ok) {
        const text = await res.text()
        throw new Error(text || `create playlist failed: ${res.status}`)
      }

      const data: CreatePlaylistOutput = await res.json()
      await refresh()
      return data
    },
    [refresh, latitude]
  )

  return useMemo(
    () => ({
      status,
      loading,
      error,
      refresh,
      connectSpotify,
      logout,
      createPlaylist,
      getPlaylists,
    }),
    [
      status,
      loading,
      error,
      refresh,
      connectSpotify,
      logout,
      createPlaylist,
      getPlaylists,
    ]
  )
}
