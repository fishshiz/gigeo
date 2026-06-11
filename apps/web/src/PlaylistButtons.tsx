import { useState } from "react"
import { useSpotifyAuth } from "./hooks/spotify"

export function PlaylistButtons() {
  const { status, loading, error, connectSpotify, createPlaylist, logout } =
    useSpotifyAuth()

  const [name, setName] = useState("My generated playlist")
  const [description, setDescription] = useState("Created by the app")
  const [isPublic, setIsPublic] = useState(false)
  const [createdUrl, setCreatedUrl] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  if (loading) return <div>Loading auth status…</div>
  if (error) return <div>Auth error: {error}</div>

  if (!status?.logged_in || !status.spotify_connected) {
    return (
      <div>
        <h2>Spotify</h2>
        <p>Your app session is missing or Spotify is not connected yet.</p>
        <button onClick={connectSpotify}>Connect Spotify</button>
      </div>
    )
  }

  const canCreate = isPublic
    ? status.can_create_public_playlist
    : status.can_create_private_playlist

  return (
    <div>
      <h2>Spotify connected</h2>
      <p>Spotify user: {status.spotify_user_id}</p>

      <label>
        Playlist name
        <input value={name} onChange={(e) => setName(e.target.value)} />
      </label>

      <label>
        Description
        <input
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </label>

      <label>
        <input
          type="checkbox"
          checked={isPublic}
          onChange={(e) => setIsPublic(e.target.checked)}
        />
        Public playlist
      </label>

      {!canCreate && (
        <p>
          Missing scope for {isPublic ? "public" : "private"} playlist creation.
          Reconnect Spotify with the required permissions.
        </p>
      )}

      <button
        disabled={submitting || !canCreate}
        onClick={async () => {
          setSubmitting(true)
          try {
            const created = await createPlaylist({
              name,
              description,
              public: isPublic,
            })
            setCreatedUrl(created.spotify_url ?? null)
          } finally {
            setSubmitting(false)
          }
        }}
      >
        {submitting ? "Creating…" : "Create playlist"}
      </button>

      <button onClick={() => void logout()}>Log out</button>

      {createdUrl && (
        <p>
          Playlist created:{" "}
          <a href={createdUrl} target="_blank" rel="noreferrer">
            Open in Spotify
          </a>
        </p>
      )}
    </div>
  )
}
