import { usePlaylistContext } from "@/providers/playlistsProvider"
import { useSpotifyAuth } from "../hooks/spotify"
import {
  CircleCheck,
  ListMusic,
  CirclePlus,
  Music2,
  ExternalLink,
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
                  <li key={playlist.id}>
                    <a
                      href={playlist.external_url ?? undefined}
                      target="_blank"
                      rel="noreferrer"
                      className="flex items-center gap-3 rounded-lg border border-black/10 p-2 transition hover:border-black/20 hover:bg-neutral-50 dark:border-white/10 dark:hover:bg-zinc-800"
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
                          {playlist.name}
                        </p>
                        <p className="truncate text-xs text-muted-foreground">
                          {playlist.track_count} track
                          {playlist.track_count === 1 ? "" : "s"}
                          {playlist.city ? ` · ${playlist.city}` : ""}
                        </p>
                      </div>
                      {playlist.external_url && (
                        <ExternalLink
                          size={14}
                          className="shrink-0 text-muted-foreground"
                        />
                      )}
                    </a>
                  </li>
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

  return (
    <div className="flex flex-col items-center justify-center">
      <Form
        className="w-full p-0!"
        action={(formData) => {
          createPlaylist(formData)
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
        <Button type="submit">Create Playlist</Button>
      </Form>
    </div>
  )
}
