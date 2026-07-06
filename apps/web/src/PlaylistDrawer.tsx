import {
  DrawerBody,
  DrawerContent,
  DrawerHeader,
  DrawerClose,
} from "@workspace/ui/components/ui/Drawer"

export const PlaylistDrawer = ({
    drawerOpen,
    setDrawerOpen,
}: {
    drawerOpen: boolean,
    setDrawerOpen: (open: boolean) => void
}) => {
return <DrawerContent closeDrawer={ () => setDrawerOpen(false)} isOpen={drawerOpen} side="right">
    <DrawerHeader>
        Playlists
    </DrawerHeader>
    <DrawerBody>
        This is where playlist info goes
    </DrawerBody>
</DrawerContent>
}