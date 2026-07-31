import {
  usePlaylistContext,
  type SpotifyPlaylist,
} from "@/providers/playlistsProvider"
import {
  useSpotifyAuth,
  parseUpdatePlaylistForm,
  PlaylistUnavailableError,
  type CreatePlaylistOutput,
} from "../hooks/spotify"
import {
  CircleCheck,
  ListMusic,
  CirclePlus,
  Music2,
  ExternalLink,
  MoreVertical,
  TriangleAlert,
} from "lucide-react"

import { Button } from "@workspace/ui/components/ui/Button"
import { RadioGroup, Radio } from "@workspace/ui/components/ui/RadioGroup"
import {
  Tabs,
  TabList,
  Tab,
  TabPanels,
  TabPanel,
} from "@workspace/ui/components/ui/Tabs"
import { Form } from "@workspace/ui/components/ui/Form"
import { Label } from "@workspace/ui/components/ui/Field"
import { useSearchProvider } from "@/providers/searchProvider"
import { MapPin, Lock, Unlock } from "lucide-react"
import { TextField } from "@workspace/ui/components/ui/TextField"
import { ToggleButton } from "@workspace/ui/components/ui/ToggleButton"
import { useState } from "react"
import { PlaylistButtons } from "../PlaylistButtons"
import { getRandomPlaylistName } from "@/lib/playlistNames"

import { DrawerBody, DrawerHeader } from "@workspace/ui/components/ui/Drawer"
import { Modal } from "@workspace/ui/components/ui/Modal"
import { Dialog } from "@workspace/ui/components/ui/Dialog"
import { AlertDialog } from "@workspace/ui/components/ui/AlertDialog"
import {
  MenuTrigger,
  Menu,
  MenuItem,
} from "@workspace/ui/components/ui/Menu"

export const PlaylistDrawerHeader = () => {
  return (
    <DrawerHeader className="sticky top-0 z-10 my-2 w-full bg-background">
      <PlaylistButtons />
    </DrawerHeader>
  )
}

export const PlaylistsDrawerBody = () => {
  const { spotifyPlaylists } = usePlaylistContext()
  const { status } = useSpotifyAuth()
  const isConnected = Boolean(status?.spotify_connected)

  return (
    <DrawerBody>
      <Tabs>
        <TabList aria-label="Playlist actions" className="mb-2">
          <Tab id="playlists" className="gap-1.5">
            <ListMusic size={15} />
            My Playlists
          </Tab>
          <Tab id="add-playlist" className="gap-1.5">
            <CirclePlus size={15} />
            Create
          </Tab>
        </TabList>
        <TabPanels>
          <TabPanel id="playlists">
            {spotifyPlaylists.length > 0 ? (
              <ul className="flex flex-col gap-2">
                {spotifyPlaylists.map((playlist) => (
                  <PlaylistListItem key={playlist.playlist_id} playlist={playlist} />
                ))}
              </ul>
            ) : (
              <div className="flex flex-col items-center justify-center gap-2 p-6 text-center">
                <ListMusic size={24} className="text-muted-foreground" />
                <p className="text-sm text-muted-foreground">
                  {isConnected
                    ? "Gigeo isn't managing any playlists yet."
                    : "Connect Spotify to see the playlists Gigeo manages for you."}
                </p>
              </div>
            )}
          </TabPanel>
          <TabPanel id="add-playlist">
            <CreatePlaylistForm />
          </TabPanel>
        </TabPanels>
      </Tabs>
    </DrawerBody>
  )
}

const cadenceToFrequencyValue = (cadence: 7 | 30 | 60) =>
  cadence === 60 ? "bimonthly" : cadence === 7 ? "weekly" : "monthly"

const PlaylistListItem = ({ playlist }: { playlist: SpotifyPlaylist }) => {
  const { deletePlaylist } = useSpotifyAuth()
  const [isEditOpen, setIsEditOpen] = useState(false)
  const [isDeleteOpen, setIsDeleteOpen] = useState(false)
  const isUnavailable = !playlist.is_active

  return (
    <li>
      <div
        className={`flex items-center gap-3 rounded-lg border border-black/10 p-2 dark:border-white/10 ${
          isUnavailable ? "opacity-50" : "hover:border-black/20 hover:bg-neutral-50 dark:hover:bg-zinc-800"
        }`}
      >
        <a
          href={isUnavailable ? undefined : (playlist.external_url ?? undefined)}
          target="_blank"
          rel="noreferrer"
          aria-disabled={isUnavailable}
          className={`flex min-w-0 flex-1 items-center gap-3 ${isUnavailable ? "pointer-events-none" : ""}`}
        >
          {playlist.images && playlist.images.length > 0 ? (
            <img
              src={playlist.images[0].url}
              alt=""
              className="h-12 w-12 shrink-0 rounded object-cover"
            />
          ) : (
            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded bg-neutral-100 text-muted-foreground dark:bg-neutral-800">
              <Music2 size={18} />
            </div>
          )}
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold text-primary">
              {playlist.name || playlist.city}
            </p>
            {isUnavailable ? (
              <p className="truncate text-xs text-red-500">
                Unavailable — removed on Spotify
              </p>
            ) : (
              <p className="truncate text-xs text-muted-foreground">
                {playlist.track_count} track
                {playlist.track_count === 1 ? "" : "s"}
                {playlist.city ? ` · ${playlist.city}` : ""}
              </p>
            )}
          </div>
          {!isUnavailable && playlist.external_url && (
            <ExternalLink size={14} className="shrink-0 text-muted-foreground" />
          )}
        </a>

        <MenuTrigger>
          <Button
            variant="secondary"
            aria-label={`Actions for ${playlist.name || "playlist"}`}
            className="shrink-0 p-1.5!"
          >
            <MoreVertical size={16} />
          </Button>
          <Menu
            onAction={(key) => {
              if (key === "edit") setIsEditOpen(true)
              if (key === "delete") setIsDeleteOpen(true)
            }}
          >
            {!isUnavailable && <MenuItem id="edit">Edit</MenuItem>}
            <MenuItem id="delete">
              {isUnavailable ? "Remove from Gigeo" : "Delete"}
            </MenuItem>
          </Menu>
        </MenuTrigger>
      </div>

      <Modal isOpen={isEditOpen} onOpenChange={setIsEditOpen} isDismissable>
        <Dialog>
          {({ close }) => (
            <EditPlaylistForm playlist={playlist} onDone={close} />
          )}
        </Dialog>
      </Modal>

      <Modal isOpen={isDeleteOpen} onOpenChange={setIsDeleteOpen} isDismissable>
        <AlertDialog
          title={isUnavailable ? "Remove from Gigeo?" : "Delete playlist?"}
          variant="destructive"
          actionLabel={isUnavailable ? "Remove" : "Delete"}
          onAction={() => {
            void deletePlaylist(playlist.playlist_id)
          }}
        >
          {isUnavailable
            ? "This playlist was already removed on Spotify. This just cleans up Gigeo's record of it."
            : "This unfollows the playlist on Spotify and removes it from Gigeo. This can't be undone."}
        </AlertDialog>
      </Modal>
    </li>
  )
}

const EditPlaylistForm = ({
  playlist,
  onDone,
}: {
  playlist: SpotifyPlaylist
  onDone: () => void
}) => {
  const { updatePlaylist } = useSpotifyAuth()
  const [isPrivate, setIsPrivate] = useState(playlist.visibility === "private")
  const [selectedFrequency, setSelectedFrequency] = useState(
    cadenceToFrequencyValue(playlist.update_cadence_days)
  )
  const [selectedBehavior, setSelectedBehavior] = useState(playlist.update_mode)
  const [error, setError] = useState<string | null>(null)

  const showDestructiveWarning =
    selectedBehavior === "destructive" && playlist.update_mode === "additive"

  return (
    <Form
      className="w-full p-0!"
      action={async (formData) => {
        setError(null)
        try {
          await updatePlaylist(
            playlist.playlist_id,
            parseUpdatePlaylistForm(formData)
          )
          onDone()
        } catch (err) {
          setError(
            err instanceof PlaylistUnavailableError
              ? "This playlist was just removed on Spotify, so it can no longer be edited."
              : err instanceof Error
                ? err.message
                : "Failed to update playlist"
          )
        }
      }}
    >
      <input
        type="hidden"
        name="privacy"
        value={isPrivate ? "private" : "public"}
      />
      <TextField
        label="Playlist Name"
        isRequired
        name="playlistName"
        defaultValue={playlist.name}
      />
      <Label>Playlist privacy</Label>
      <ToggleButton
        aria-label="Make playlist private"
        isSelected={isPrivate}
        onChange={setIsPrivate}
      >
        {isPrivate ? <Lock size={18} /> : <Unlock size={18} />}
      </ToggleButton>
      <p>{isPrivate ? "Private" : "Public"}</p>

      <Label>Update frequency</Label>
      <RadioGroup
        aria-label="Playlist update frequency"
        value={selectedFrequency}
        onChange={setSelectedFrequency}
        orientation="horizontal"
        className="flex w-full gap-2.5"
        name="cadence"
        isRequired
      >
        {updateFrequencyOptions.map((option) => (
          <RadioCard
            key={option.value}
            value={option.value}
            title={option.title}
            description={option.description}
          />
        ))}
      </RadioGroup>
      <Label>Update behavior</Label>
      <RadioGroup
        aria-label="Playlist update behavior"
        value={selectedBehavior}
        onChange={(value) =>
          setSelectedBehavior(value as "additive" | "destructive")
        }
        orientation="horizontal"
        className="flex w-full gap-2.5"
        name="behavior"
        isRequired
      >
        {updateBehaviorOptions.map((option) => (
          <RadioCard
            key={option.value}
            value={option.value}
            title={option.title}
            description={option.description}
          />
        ))}
      </RadioGroup>
      {showDestructiveWarning && (
        <p className="flex items-start gap-1.5 text-xs text-amber-600 dark:text-amber-400">
          <TriangleAlert size={14} className="mt-0.5 shrink-0" />
          Switching to "Replace tracks" may remove tracks you've kept.
          Changes apply on the playlist's next scheduled update, not
          immediately.
        </p>
      )}
      {error && <p className="text-xs text-red-500">{error}</p>}
      <Button type="submit">Save changes</Button>
    </Form>
  )
}

const updateFrequencyOptions = [
  { value: "weekly", title: "Weekly", description: "Every 7 days" },
  { value: "monthly", title: "Monthly", description: "Every 30 days" },
  { value: "bimonthly", title: "Bimonthly", description: "Every 60 days" },
]

const updateBehaviorOptions = [
  {
    value: "destructive",
    title: "Replace tracks",
    description:
      "Rebuild playlist from scratch on each update, keeping playlist light and fresh.",
  },
  {
    value: "additive",
    title: "Add new tracks",
    description: "New tracks are added and old ones are preserved.",
  },
]

const radioCardClassName =
  "relative min-h-20 flex-1 cursor-default gap-1 rounded-lg border border-black/10 bg-white/80 p-3 text-left text-slate-700 transition select-none before:hidden before:content-none hover:border-black/20 hover:shadow-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 dark:border-white/10 dark:bg-zinc-900/70 dark:text-slate-200 dark:hover:border-white/20 selected:border-primary selected:bg-primary/5 selected:shadow-md"

const RadioCard = ({
  value,
  title,
  description,
}: {
  value: string
  title: string
  description: string
}) => (
  <Radio value={value} className={radioCardClassName}>
    {({ isSelected }) => (
      <>
        <div className="min-w-0 pr-5">
          <div className="text-sm font-semibold text-primary">{title}</div>
          <span slot="description" className="text-xs text-muted-foreground">
            {description}
          </span>
        </div>

        {isSelected && (
          <span
            aria-hidden="true"
            className="absolute top-3 right-3 text-primary"
          >
            <CircleCheck size={16} />
          </span>
        )}
      </>
    )}
  </Radio>
)

export const CreatePlaylistForm = () => {
  const { focusSearchInput, selectedLocation } = useSearchProvider()
  const { createPlaylist } = useSpotifyAuth()

  const emptyLocationString = "Search a location to start"
  const placeholder = getRandomPlaylistName(
    selectedLocation?.split(",")[0] || ""
  )
  const [isPrivate, setIsPrivate] = useState(false)
  const [selectedFrequency, setSelectedFrequency] = useState("weekly")
  const [selectedBehavior, setSelectedBehavior] = useState("destructive")
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<CreatePlaylistOutput | null>(null)

  return (
    <div className="flex flex-col items-center justify-center">
      <Form
        className="w-full p-0!"
        action={async (formData) => {
          setError(null)
          setSuccess(null)
          setIsSubmitting(true)
          try {
            const result = await createPlaylist(formData)
            setSuccess(result)
          } catch (err) {
            setError(
              err instanceof Error ? err.message : "Failed to create playlist"
            )
          } finally {
            setIsSubmitting(false)
          }
        }}
      >
        <input type="hidden" name="location" value={selectedLocation ?? ""} />
        <input
          type="hidden"
          name="privacy"
          value={isPrivate ? "private" : "public"}
        />
        <Label>Playlist location</Label>
        <Button
          aria-label="Playlist location"
          variant="secondary"
          className="relative flex w-full! justify-start gap-2 p-4! text-left"
          onClick={() => focusSearchInput()}
          name="location"
        >
          <MapPin
            className="lucide lucide-map-pin pointer-events-none text-muted-foreground"
            size={15}
          />
          {selectedLocation || emptyLocationString}
        </Button>
        <TextField
          label="Playlist Name"
          placeholder={placeholder}
          isRequired
          validate={(value) => (value === "admin" ? "Nice try." : null)}
          name="playlistName"
          defaultValue={placeholder}
        />
        <Label>Playlist privacy</Label>
        <ToggleButton
          aria-label="Make playlist private"
          isSelected={isPrivate}
          onChange={setIsPrivate}
        >
          {isPrivate ? <Lock size={18} /> : <Unlock size={18} />}
        </ToggleButton>
        <p>{isPrivate ? "Private" : "Public"}</p>

        <Label>Update frequency</Label>
        <RadioGroup
          aria-label="Playlist update frequency"
          value={selectedFrequency}
          onChange={setSelectedFrequency}
          orientation="horizontal"
          className="flex w-full gap-2.5"
          name="cadence"
          isRequired
        >
          {updateFrequencyOptions.map((option) => (
            <RadioCard
              key={option.value}
              value={option.value}
              title={option.title}
              description={option.description}
            />
          ))}
        </RadioGroup>
        <Label>Update behavior</Label>
        <RadioGroup
          aria-label="Playlist update behavior"
          value={selectedBehavior}
          onChange={setSelectedBehavior}
          orientation="horizontal"
          className="flex w-full gap-2.5"
          name="behavior"
          isRequired
        >
          {updateBehaviorOptions.map((option) => (
            <RadioCard
              key={option.value}
              value={option.value}
              title={option.title}
              description={option.description}
            />
          ))}
        </RadioGroup>
        {success && (
          <p className="flex items-center gap-1.5 text-xs text-emerald-600 dark:text-emerald-400">
            <CircleCheck size={14} className="shrink-0" />
            "{success.name}" created
            {success.spotify_url && (
              <>
                {" · "}
                <a
                  href={success.spotify_url}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 underline"
                >
                  Open in Spotify
                  <ExternalLink size={12} />
                </a>
              </>
            )}
          </p>
        )}
        {error && <p className="text-xs text-red-500">{error}</p>}
        <Button type="submit" isPending={isSubmitting} isDisabled={isSubmitting}>
          Create Playlist
        </Button>
      </Form>
    </div>
  )
}
