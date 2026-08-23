import { useCallback, useEffect, useMemo, useState } from "react"
import {
  usePlaylistContext,
  type SpotifyPlaylist,
} from "@/providers/playlistsProvider"
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
  privacy: boolean
  cadence: number
  radius: number
  destructive: boolean
  genres: string[]
}

export type CreatePlaylistOutput = {
  id: string
  name: string
  spotify_url?: string | null
}

export type UpdatePlaylistInput = {
  name: string
  privacy: boolean
  cadence: number
  destructive: boolean
  genres: string[]
}

/** Thrown when the server confirms a playlist is no longer on Spotify (410). */
export class PlaylistUnavailableError extends Error {
  constructor() {
    super("Playlist is no longer available on Spotify")
    this.name = "PlaylistUnavailableError"
  }
}

type UseSpotifyAuthResult = {
  status: AuthStatus | null
  loading: boolean
  error: string | null
  refresh: () => Promise<void>
  connectSpotify: () => void
  logout: () => Promise<void>
  createPlaylist: (input: FormData) => Promise<CreatePlaylistOutput>
  getPlaylists: () => Promise<SpotifyPlaylist[]>
  updatePlaylist: (
    playlistId: string,
    input: UpdatePlaylistInput
  ) => Promise<void>
  deletePlaylist: (playlistId: string) => Promise<void>
}

/** Reads the `genres` hidden input (a JSON-stringified string array, kept
 * in sync with the genre checkbox picker the same way `privacy` mirrors its
 * toggle). Missing or malformed input means "no filter", never a thrown
 * validation error -- unlike name/location/cadence, an empty genre list is
 * a fully valid, common choice. */
function parseGenresField(formData: FormData): string[] {
  const raw = formData.get("genres")
  if (typeof raw !== "string" || !raw) return []
  try {
    const parsed: unknown = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed.filter((g) => typeof g === "string") : []
  } catch {
    return []
  }
}

export function parseCreatePlaylistForm(
  formData: FormData
): CreatePlaylistInput {
  const name = formData.get("playlistName")
  const location = formData.get("location")
  const privacy = formData.get("privacy")
  const cadence = formData.get("cadence")
  const behavior = formData.get("behavior")

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
    privacy: privacy === "private",
    cadence: cadence === "bimonthly" ? 60 : cadence === "weekly" ? 7 : 30,
    destructive: behavior === "destructive",
    radius: 25,
    genres: parseGenresField(formData),
  }
}

export function parseUpdatePlaylistForm(formData: FormData): UpdatePlaylistInput {
  const name = formData.get("playlistName")
  const privacy = formData.get("privacy")
  const cadence = formData.get("cadence")
  const behavior = formData.get("behavior")

  if (typeof name !== "string" || !name.trim()) {
    throw new Error("Playlist name is required")
  }

  if (typeof cadence !== "string" || !cadence) {
    throw new Error("Cadence is required")
  }

  return {
    name: name.trim(),
    privacy: privacy === "private",
    cadence: cadence === "bimonthly" ? 60 : cadence === "weekly" ? 7 : 30,
    destructive: behavior === "destructive",
    genres: parseGenresField(formData),
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
    // Fetch auth status on mount; intentional, not a cascading-render risk here.
    // eslint-disable-next-line react-hooks/set-state-in-effect
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

    const res: SpotifyPlaylist[] = await fetch(`api/spotify/playlist`, {
      method: "GET",
      credentials: "include",
    }).then((res) => res.json())
    setSpotifyPlaylists(res)
    setPlaylistManagement("spotify")
    return res
  }, [setSpotifyPlaylists, setPlaylistManagement])

  useEffect(() => {
    // Fetch the user's playlists once Spotify is connected, so "My
    // Playlists" has data without requiring a manual trigger.
    if (status?.spotify_connected) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      void getPlaylists()
    }
  }, [status?.spotify_connected, getPlaylists])

  const createPlaylist = useCallback(
    async (input: FormData) => {
      setError(null)
      const payload = parseCreatePlaylistForm(input)

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
    [refresh, latitude, longitude]
  )

  const updatePlaylist = useCallback(
    async (playlistId: string, input: UpdatePlaylistInput) => {
      setError(null)

      const res = await fetch(`api/spotify/playlist/${playlistId}`, {
        method: "PATCH",
        credentials: "include",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(input),
      })

      if (res.status === 410) {
        // Confirmed gone on Spotify's side — refresh so the row flips to
        // "unavailable", then let the caller distinguish this from a
        // generic failure.
        await getPlaylists()
        throw new PlaylistUnavailableError()
      }

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

      const res = await fetch(`api/spotify/playlist/${playlistId}`, {
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
      connectSpotify,
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
      connectSpotify,
      logout,
      createPlaylist,
      getPlaylists,
      updatePlaylist,
      deletePlaylist,
    ]
  )
}
