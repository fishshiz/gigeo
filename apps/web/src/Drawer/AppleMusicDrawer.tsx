import {
  usePlaylistContext,
  type ApplePlaylist,
} from "@/providers/playlistsProvider"
import {
  useAppleMusicAuth,
  parseUpdateApplePlaylistForm,
} from "../hooks/appleMusic"
import {
  CircleCheck,
  ListMusic,
  CirclePlus,
  Music2,
  MoreVertical,
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
import { MapPin } from "lucide-react"
import { TextField } from "@workspace/ui/components/ui/TextField"
import { useState } from "react"
import { AppleMusicButtons } from "../AppleMusicButtons"
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

export const AppleMusicDrawerHeader = () => {
  return (
    <DrawerHeader className="sticky top-0 z-10 my-2 w-full bg-background">
      <AppleMusicButtons />
    </DrawerHeader>
  )
}

export const AppleMusicDrawerBody = () => {
  const { applePlaylists } = usePlaylistContext()
  const { status } = useAppleMusicAuth()
  const isConnected = Boolean(status?.apple_music_connected)

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
            {applePlaylists.length > 0 ? (
              <ul className="flex flex-col gap-2">
                {applePlaylists.map((playlist) => (
                  <PlaylistListItem key={playlist.playlist_id} playlist={playlist} />
                ))}
              </ul>
            ) : (
              <div className="flex flex-col items-center justify-center gap-2 p-6 text-center">
                <ListMusic size={24} className="text-muted-foreground" />
                <p className="text-sm text-muted-foreground">
                  {isConnected
                    ? "Gigeo isn't managing any playlists yet."
                    : "Connect Apple Music to see the playlists Gigeo manages for you."}
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

const PlaylistListItem = ({ playlist }: { playlist: ApplePlaylist }) => {
  const { deletePlaylist } = useAppleMusicAuth()
  const [isEditOpen, setIsEditOpen] = useState(false)
  const [isDeleteOpen, setIsDeleteOpen] = useState(false)
  const needsReconnect = !playlist.is_active

  return (
    <li>
      <div
        className={`flex items-center gap-3 rounded-lg border border-black/10 p-2 dark:border-white/10 ${
          needsReconnect ? "opacity-50" : "hover:border-black/20 hover:bg-neutral-50 dark:hover:bg-zinc-800"
        }`}
      >
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded bg-neutral-100 text-muted-foreground dark:bg-neutral-800">
          <Music2 size={18} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold text-primary">
            {playlist.name || playlist.city}
          </p>
          {needsReconnect ? (
            <p className="truncate text-xs text-amber-600 dark:text-amber-400">
              Unavailable — reconnect Apple Music to resume updates
            </p>
          ) : (
            <p className="truncate text-xs text-muted-foreground">
              {playlist.track_count} track
              {playlist.track_count === 1 ? "" : "s"}
              {playlist.city ? ` · ${playlist.city}` : ""}
            </p>
          )}
        </div>

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
            {!needsReconnect && <MenuItem id="edit">Edit</MenuItem>}
            <MenuItem id="delete">Remove from Gigeo</MenuItem>
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
          title="Remove from Gigeo?"
          variant="destructive"
          actionLabel="Remove"
          onAction={() => {
            void deletePlaylist(playlist.playlist_id)
          }}
        >
          Apple Music doesn't let apps delete playlists — this only stops
          Gigeo from updating it. The playlist stays in your Apple Music
          library exactly as it is now.
        </AlertDialog>
      </Modal>
    </li>
  )
}

const EditPlaylistForm = ({
  playlist,
  onDone,
}: {
  playlist: ApplePlaylist
  onDone: () => void
}) => {
  const { updatePlaylist } = useAppleMusicAuth()
  const [selectedFrequency, setSelectedFrequency] = useState(
    cadenceToFrequencyValue(playlist.update_cadence_days)
  )
  const [error, setError] = useState<string | null>(null)

  return (
    <Form
      className="w-full p-0!"
      action={async (formData) => {
        setError(null)
        try {
          await updatePlaylist(
            playlist.playlist_id,
            parseUpdateApplePlaylistForm(formData)
          )
          onDone()
        } catch (err) {
          setError(err instanceof Error ? err.message : "Failed to update playlist")
        }
      }}
    >
      <p className="text-xs text-muted-foreground">
        Apple Music playlists can only add new tracks, never remove them —
        update frequency is the only thing to configure here.
      </p>
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
  const { createPlaylist } = useAppleMusicAuth()

  const emptyLocationString = "Search a location to start"
  const placeholder = getRandomPlaylistName(selectedLocation?.cityName || "")
  const [selectedFrequency, setSelectedFrequency] = useState("weekly")
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<{ name: string } | null>(null)

  return (
    <div className="flex flex-col items-center justify-center">
      <Form
        className="w-full p-0!"
        action={async (formData) => {
          setError(null)
          setSuccess(null)
          setIsSubmitting(true)
          try {
            await createPlaylist(formData)
            setSuccess({ name: (formData.get("playlistName") as string) || "" })
          } catch (err) {
            setError(
              err instanceof Error ? err.message : "Failed to create playlist"
            )
          } finally {
            setIsSubmitting(false)
          }
        }}
      >
        <input
          type="hidden"
          name="location"
          value={selectedLocation?.fullAddress ?? ""}
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
          {selectedLocation?.fullAddress || emptyLocationString}
        </Button>
        <TextField
          label="Playlist Name"
          placeholder={placeholder}
          isRequired
          validate={(value) => (value === "admin" ? "Nice try." : null)}
          name="playlistName"
          defaultValue={placeholder}
        />
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
        <p className="text-xs text-muted-foreground">
          Updates only ever add new tracks — Apple Music doesn't let apps
          remove them.
        </p>
        {success && (
          <p className="flex items-center gap-1.5 text-xs text-emerald-600 dark:text-emerald-400">
            <CircleCheck size={14} className="shrink-0" />
            "{success.name}" created
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
