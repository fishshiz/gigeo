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

  const connectSpotify = useCallback(async () => {
    try {
      // 1. Perform the fetch request
      await fetch("/api/spotify/login", {
        redirect: "follow",
        mode: "no-cors",
      }).then((res) => {
        console.log("Final URL:", res) // Log the final URL for debugging
        if (!res.ok) {
          throw new Error(`Response status: ${res.status}`)
        }
        // 2. Get the final URL after any redirects
        const finalUrl = res.url

        // 3. Open the final URL in a new tab
        // Note: This must be triggered by a direct user action (like a button click handler)
        // to avoid being blocked by the browser's pop-up blocker.
        const newTab = window.open(finalUrl, "_blank")
        if (newTab) {
          newTab.focus() // Focus the newly opened tab
        } else {
          alert(
            "Pop-up blocked. Please allow pop-ups for this site to open the link in a new tab."
          )
        }
      })
    } catch (error) {
      console.error("Error during fetch:", error)
    }
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
    }),
    [status, loading, error, refresh, connectSpotify, logout, createPlaylist]
  )
}
