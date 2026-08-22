/* eslint-disable react-refresh/only-export-components */
import { createContext, useState, useContext } from "react"

export type SpotifyPlaylistImage = {
  url: string
  height?: number | null
  width?: number | null
}

export type SpotifyPlaylist = {
  playlist_id: string
  id: string
  name: string
  external_url?: string | null
  images: SpotifyPlaylistImage[]
  track_count: number
  city: string
  visibility: "public" | "private"
  update_mode: "additive" | "destructive"
  update_cadence_days: 7 | 30 | 60
  is_active: boolean
  genres: string[]
}

/// Apple's public API only supports create + add-tracks (see Phase 0's
/// plan doc) — there's no rename, visibility toggle, or destructive
/// update mode to report, so this is intentionally a narrower shape than
/// `SpotifyPlaylist` rather than padding it out with always-constant
/// fields.
export type ApplePlaylist = {
  playlist_id: string
  id: string
  name: string
  track_count: number
  city: string
  update_cadence_days: 7 | 30 | 60
  is_active: boolean
  genres: string[]
}

type PlaylistProviderState = {
  spotifyPlaylists: SpotifyPlaylist[]
  setSpotifyPlaylists: (playlists: SpotifyPlaylist[]) => void
  applePlaylists: ApplePlaylist[]
  setApplePlaylists: (playlists: ApplePlaylist[]) => void
  playlistManagement: "spotify" | "apple" | undefined
  setPlaylistManagement: (management: "spotify" | "apple" | undefined) => void
}

export const PlaylistProviderContext = createContext<
  PlaylistProviderState | undefined
>(undefined)

type PlaylistProviderProps = {
  children: React.ReactNode
}

export function PlaylistProvider({ children }: PlaylistProviderProps) {
  const [spotifyPlaylists, setSpotifyPlaylists] = useState<SpotifyPlaylist[]>(
    []
  )
  const [applePlaylists, setApplePlaylists] = useState<ApplePlaylist[]>([])
  const [playlistManagement, setPlaylistManagement] = useState<
    "spotify" | "apple" | undefined
  >(undefined)

  return (
    <PlaylistProviderContext.Provider
      value={{
        spotifyPlaylists,
        setSpotifyPlaylists,
        applePlaylists,
        setApplePlaylists,
        playlistManagement,
        setPlaylistManagement,
      }}
    >
      {children}
    </PlaylistProviderContext.Provider>
  )
}

export function usePlaylistContext() {
  const ctx = useContext(PlaylistProviderContext)
  if (!ctx) {
    throw new Error("usePlaylistContext must be used within PlaylistProvider")
  }
  return ctx
}
