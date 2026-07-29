import { usePlaylistContext } from "@/providers/playlistsProvider"
import { useSpotifyAuth } from "../hooks/spotify"
import { CircleCheck } from "lucide-react"

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

import { DrawerBody, DrawerHeader } from "@workspace/ui/components/ui/Drawer"

export const PlaylistDrawerHeader = () => {
  return (
    <DrawerHeader className="sticky top-0 z-10 my-2 w-full bg-white">
      <PlaylistButtons />
    </DrawerHeader>
  )
}

export const PlaylistsDrawerBody = () => {
  const { spotifyPlaylists } = usePlaylistContext()

  return (
    <DrawerBody>
      <Tabs>
        <TabList>
          <Tab id="playlists">My Playlists</Tab>
          <Tab id="add-playlist">Create</Tab>
        </TabList>
        <TabPanels>
          <TabPanel id="playlists">
            <div className="flex flex-col gap-2">
              {spotifyPlaylists.length > 0 ? (
                <div>
                  <h2>Spotify Playlists</h2>
                  <ul>
                    {spotifyPlaylists.map((playlist) => (
                      <li
                        key={playlist.id}
                        className="flex items-center gap-2 rounded-lg border border-gray-300 p-2 hover:bg-gray-100"
                      >
                        {playlist.images && playlist.images.length > 0 && (
                          <img
                            src={playlist.images[0].url}
                            alt={playlist.name}
                            className="h-12 w-12 rounded"
                          />
                        )}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center gap-4 p-4">
                  <p>Gigeo is not managing any playlists.</p>
                </div>
              )}
            </div>
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
            <Radio
              key={option.value}
              value={option.value}
              className="relative min-h-20 flex-1 cursor-default gap-1 rounded-lg border border-black/10 bg-white/80 p-3 text-left text-slate-700 transition select-none before:hidden before:content-none hover:border-black/20 hover:shadow-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 dark:border-white/10 dark:bg-zinc-900/70 dark:text-slate-200 dark:hover:border-white/20 selected:border-primary selected:bg-primary/5 selected:shadow-md"
            >
              {({ isSelected }) => (
                <>
                  <div className="min-w-0 pr-5">
                    <div className="text-sm font-semibold text-primary">
                      {option.title}
                    </div>
                    <span
                      slot="description"
                      className="text-xs text-muted-foreground"
                    >
                      {option.description}
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
          <Radio
            key="destructive"
            value="destructive"
            className="relative min-h-20 flex-1 cursor-default gap-1 rounded-lg border border-black/10 bg-white/80 p-3 text-left text-slate-700 transition select-none before:hidden before:content-none hover:border-black/20 hover:shadow-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 dark:border-white/10 dark:bg-zinc-900/70 dark:text-slate-200 dark:hover:border-white/20 selected:border-primary selected:bg-primary/5 selected:shadow-md"
          >
            {({ isSelected }) => (
              <>
                <div className="min-w-0 pr-5">
                  <div className="text-sm font-semibold text-primary">
                    Replace tracks
                  </div>
                  <span
                    slot="description"
                    className="text-xs text-muted-foreground"
                  >
                    Rebuild playlist from scratch on each update, keeping
                    playlist light and fresh.
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
          <Radio
            key="additive"
            value="additive"
            className="relative min-h-20 flex-1 cursor-default gap-1 rounded-lg border border-black/10 bg-white/80 p-3 text-left text-slate-700 transition select-none before:hidden before:content-none hover:border-black/20 hover:shadow-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 dark:border-white/10 dark:bg-zinc-900/70 dark:text-slate-200 dark:hover:border-white/20 selected:border-primary selected:bg-primary/5 selected:shadow-md"
          >
            {({ isSelected }) => (
              <>
                <div className="min-w-0 pr-5">
                  <div className="text-sm font-semibold text-primary">
                    Add new tracks
                  </div>
                  <span
                    slot="description"
                    className="text-xs text-muted-foreground"
                  >
                    New tracks are added and old ones are preserved.
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
        </RadioGroup>
        <Button type="submit">Create Playlist</Button>
      </Form>
    </div>
  )
}

function getRandomPlaylistName(city: string) {
  const templates = [
    `Live, from ${city}`,
    `Tonight in ${city}`,
    `${city} After Dark`,
    `Sounds of ${city}`,
    `On Stage in ${city}`,
    `${city} Nightlights`,
    `From the Heart of ${city}`,
    `${city}, Amplified`,
    `Late Nights in ${city}`,
    `Live and Loud in ${city}`,
  ]

  return city.length > 0
    ? templates[Math.floor(Math.random() * templates.length)]
    : ""
}
