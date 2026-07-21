import { createContext, useState, useContext } from "react"
type PlaylistProviderState = {
  spotifyPlaylists: any[]
  setSpotifyPlaylists: (playlists: any[]) => void
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
  const [spotifyPlaylists, setSpotifyPlaylists] = useState<any[]>([])
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
