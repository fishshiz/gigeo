import { useCallback, useEffect, useMemo, useState } from "react"
export type AuthStatus = {
  logged_in: boolean
  spotify_connected: boolean
  spotify_user_id: string | null
  can_create_private_playlist: boolean
  can_create_public_playlist: boolean
}

export type CreatePlaylistInput = {
  name: string
  description?: string
  public?: boolean
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
  createPlaylist: (input: CreatePlaylistInput) => Promise<CreatePlaylistOutput>
  getPlaylists: () => Promise<any>
}

export function useSpotifyAuth(): UseSpotifyAuthResult {
  const [status, setStatus] = useState<AuthStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

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
  window.location.assign("/api/spotify/login");
}, []);

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
    
    const res = await fetch(`api/spotify/playlists`, {
      method: "GET",
      credentials: "include",
    })
    console.log(res);
  }, [refresh])

  const createPlaylist = useCallback(
    async (input: CreatePlaylistInput) => {
      setError(null)

      const res = await fetch(`api/spotify/create-playlist`, {
        method: "POST",
        credentials: "include",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          ...input,
          public: input.public ?? false,
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
    [refresh]
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
      getPlaylists
    }),
    [status, loading, error, refresh, connectSpotify, logout, createPlaylist, getPlaylists]
  )
}
