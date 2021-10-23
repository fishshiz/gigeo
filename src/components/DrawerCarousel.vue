<script setup lang="ts">
import { GeocodeFeature, SpotifyArtist, TMEvent, Coordinates } from "../interface";
import 'flag-icon-css/css/flag-icon.css'
import DrawerItem from './DrawerItem.vue';
import SearchWrapper from './SearchWrapper.vue'
import Icon from './Icon.vue';
import { reactive, inject } from 'vue'

interface Props {
    events: TMEvent[],
    selectedEvents: TMEvent[],
    selectedVenue: string,
    drawerOpen: boolean,
}

defineProps<Props>();

const emit = defineEmits<{
    (e: 'geocode', obj: { geocode: GeocodeFeature, dateRange: [Date, Date] }): void,
    (e: 'artist', item: SpotifyArtist): void,
    (e: 'hover', id: string | null): void,
    (e: 'item-click', location: Coordinates): void,
    (e: 'clear'): void,
    (e: "toggle"): void
}>()

function toggleDrawer() {
    emit('toggle')
}

function emitSelect(e: SpotifyArtist | GeocodeFeature) {
    if (e.type === 'artist') {
        //@ts-ignore
        emit('artist', e)
    } else {
        //@ts-ignore
        emit('geocode', e)
    }
}

function emitMouseOver(id: string | null) {
    emit('hover', id);
}

function emitClick(event: TMEvent) {
    const { location } = event._embedded.venues[0];
    emit('item-click', location)
}

function emitClear(): void {
    emit('clear');
}
</script>

<template>
    <div :class="[{ 'pane-collapsed': !drawerOpen }, 'pane']">
        <div class="pane-content">
            <div class="pane-content-holder">
                <SearchWrapper @artist="emitSelect" @geocode="emitSelect" />
                <div class="scrollbox edan" v-if="selectedEvents.length">
                    <div class="selected-header">
                        <Icon
                            :size="15"
                            color="var(--dynamic-title-color)"
                            icon="arrow_back"
                            class="back-btn"
                            @click="emitClear"
                        />
                        {{ selectedEvents.length }} events at {{ selectedVenue }}
                    </div>
                    <div v-for="(event, idx) in selectedEvents" :id="event.id" :key="event.id">
                        <DrawerItem
                            @mouseover="emitMouseOver(event.id)"
                            @mouseleave="emitMouseOver(null)"
                            @click="emitClick(event)"
                            :title="event.name"
                            :images="event.images"
                            :time="event.dates.start.localTime"
                            :day="event.dates.start.dateTime"
                            :venue="event._embedded.venues[0]"
                            :ticket-link="event.url"
                            :price-range="event.priceRanges"
                        />
                    </div>
                </div>
                <div class="scrollbox" v-else-if="events.length">
                    <div v-for="(event, idx) in events" :id="event.id" :key="event.id">
                        <DrawerItem
                            @mouseover="emitMouseOver(event.id)"
                            @mouseleave="emitMouseOver(null)"
                            @click="emitClick(event)"
                            :title="event.name"
                            :images="event.images"
                            :time="event.dates.start.localTime"
                            :day="event.dates.start.dateTime"
                            :venue="event._embedded.venues[0]"
                            :ticket-link="event.url"
                            :price-range="event.priceRanges"
                        />
                    </div>
                </div>
                <div v-else class="empty-wrapper">
                    <h2>Nothing to see here...</h2>
                    <span>Try searching by city and date, or by individual artist</span>
                </div>
            </div>
        </div>
        <div class="pane-btn-holder">
            <button @click="toggleDrawer">
                <img class="btn-img" src="https://maps.gstatic.com/tactile/pane/arrow_left_2x.png" />
            </button>
        </div>
    </div>
</template>

<style scoped lang="scss">
.pane {
    width: 408px;
    position: absolute;
    top: 0;
    z-index: 3;
    opacity: 1;
    height: 100%;
    box-shadow: 0 0 20px rgb(0 0 0 / 30%);
    // background: $dark0;
    left: 0;
    -webkit-transform: translateX(0px);
    transform: translateX(0px);
    transition-property: -webkit-transform, transform, opacity;
    transition-duration: 0.2s;
    transition-timing-function: cubic-bezier(0, 0, 0.2, 1);
}

.pane-collapsed {
    box-shadow: none;
    -webkit-transform: translateX(-408px);
    transform: translateX(-408px);
}

.pane-content {
    position: absolute;
    z-index: 1;
    left: 0;
    top: 0;
    width: 100%;
    background-color: var(--app-background-color);
    height: 100%;
    font-family: Roboto, Arial, sans-serif;
}

.pane-content-holder {
    display: -webkit-flex;
    display: -ms-flexbox;
    display: flex;
    -webkit-flex-direction: column;
    -ms-flex-direction: column;
    flex-direction: column;
    text-align: left;
    overflow: visible;
    position: absolute;
    height: 100%;
    width: 100%;
}

.pane-btn-holder {
    position: absolute;
    z-index: 0;
    top: calc(50% - 24px);
    left: 408px;
    display: block;
}
button {
    width: 23px;
    height: 48px;
    cursor: pointer;
    // border-left: 1px solid $dark1;
    box-shadow: 0px 1px 4px rgb(0 0 0 / 30%);
    border-radius: 0 8px 8px 0;
    // background: $dark4 7px center/7px 10px no-repeat;
}

.scrollbox {
    overflow: auto;
    position: relative;
}

.btn-img {
    width: 100%;
    vertical-align: middle;
}

.empty-wrapper {
    margin: 16px;
    color: var(--dynamic-title-color);
}

.edan {
    background: var(--app-background-color);
}

.selected-header {
    background: var(--dynamic-border-color);
    display: flex;
    justify-content: flex-start;
    align-items: center;
    height: 40px;
    color: var(--button-color);
}

.back-btn {
    padding: 0 16px;
    cursor: pointer;
    color: var(--dynamic-title-color);
    width: 15px;
}
</style>