<script setup lang="ts">
import { GeocodeFeature, SpotifyArtist, TMEvent, Coordinates } from "../interface";
import 'flag-icon-css/css/flag-icon.css'
import DrawerItem from './DrawerItem.vue';
import SearchWrapper from './SearchWrapper.vue'
import DrawerSelectedItem from './DrawerSelectedItem.vue';
import Loadings from './Loading.vue';
import { computed, ref } from 'vue'

interface Props {
    events: TMEvent[],
    selectedEvents: TMEvent[],
    selectedVenue: string,
    drawerOpen: boolean,
    selectedEvent: TMEvent,
    loading: boolean,
}

const props = defineProps<Props>();

const emit = defineEmits<{
    (e: 'geocode', geocode: GeocodeFeature): void,
    (e: 'artist', name: string): void,
    (e: 'dateRange', dateRange: [Date, Date]): void,
    (e: 'hover', id: string | null): void,
    (e: 'item-click', event: TMEvent): void,
    (e: 'clear-cluster'): void,
    (e: 'toggle'): void,
    (e: 'clear'): void,
}>()

const hoverId = ref<string>('')

function toggleDrawer() {
    emit('toggle')
}

function emitGeocode(geocode: GeocodeFeature) {
    emit('geocode', geocode)
}

function emitDateRange(dateRange: any) {
    emit('dateRange', dateRange)
}

function emitArtistSelect(name: string) {
    emit('artist', name)
}

function emitMouseOver(id: string | null) {
    hoverId.value = id ? id : '';
    emit('hover', id);
}

function emitClick(event: TMEvent) {
    emit('item-click', event)
}

function emitClear(): void {
    emit('clear');
}

function emitClearCluster(): void {
    emit('clear-cluster');
}
</script>

<template>
    <div :class="[{ 'pane-collapsed': !drawerOpen }, 'pane']">
        <div class="pane-content">
            <div class="pane-content-holder">
                <SearchWrapper
                    @artist="emitArtistSelect"
                    @geocode="emitGeocode"
                    @date-range="emitDateRange"
                />
                <Loadings class="loading" v-if="loading" />
                <transition v-else name="fade" class="transition">
                    <DrawerSelectedItem
                        v-if="!!selectedEvent"
                        :event="selectedEvent"
                        @back="emitClear"
                        @artist-select="emitArtistSelect"
                    />
                    <div class="scrollbox" v-else-if="selectedEvents.length">
                        <div class="selected-header">
                            <span @click="emitClearCluster" class="material-icons back-btn">&#xe5cb;</span>
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
                                :hover="hoverId === event.id"
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
                                :hover="hoverId === event.id"
                            />
                        </div>
                    </div>
                    <div v-else class="empty-wrapper">
                        <h2 class="empty-header">Nothing to see here...</h2>
                        <span>Try searching by city and date, or by individual artist</span>
                    </div>
                </transition>
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
    background-color: var(--app-background-color);
    height: 100%;
    font-family: Roboto, Arial, sans-serif;
}

.pane-content-holder {
    display: grid;
    text-align: left;
    grid-template-columns: 100%;
    grid-template-rows: 66px auto;
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
    height: 200px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
}

.empty-header {
    font-size: 30px;
    margin-bottom: 12px;
}

.loading {
    height: 340px;
}
.transition {
    z-index: 2;
    grid-row: 2;
}
.selected-header {
    background: var(--dynamic-border-color);
    display: flex;
    justify-content: flex-start;
    align-items: center;
    height: 40px;
    color: var(--dynamic-title-color);
}

.back-btn {
    padding: 0 16px;
    cursor: pointer;
    color: var(--dynamic-title-color);
    width: 15px;
}
.fade-enter-active,
.fade-leave-active {
    transition-duration: 0.3s;
    transition-property: opacity;
    transition-timing-function: cubic-bezier(0.55, 0, 0.1, 1);
    overflow: hidden;
}

.fade-leave-active {
    opacity: 0;
}

.fade-enter {
    opacity: 0;
}
</style>