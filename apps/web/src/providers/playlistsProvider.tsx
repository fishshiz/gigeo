/* eslint-disable react-refresh/only-export-components */
import { createContext, useState, useContext } from "react"

export type SpotifyPlaylistImage = {
  url: string
  height?: number | null
  width?: number | null
}

export type SpotifyPlaylist = {
  id: string
  name: string
  external_url?: string | null
  images: SpotifyPlaylistImage[]
  track_count: number
  city: string
}

type PlaylistProviderState = {
  spotifyPlaylists: SpotifyPlaylist[]
  setSpotifyPlaylists: (playlists: SpotifyPlaylist[]) => void
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
  const [playlistManagement, setPlaylistManagement] = useState<
    "spotify" | "apple" | undefined
  >(undefined)

  return (
    <PlaylistProviderContext.Provider
      value={{
        spotifyPlaylists,
        setSpotifyPlaylists,
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
