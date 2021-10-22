<script setup lang="ts">
import Search from './Search.vue'
import Dropdown from './Dropdown.vue'
import debounce from 'lodash.debounce';
import moment from 'moment';
import Datepicker from 'vue3-date-time-picker';
import 'vue3-date-time-picker/dist/main.css'
import { reactive, inject, computed, onMounted, nextTick, watch } from 'vue'
import { GeocodeResponse, GeocodeFeature, SpotifyArtist } from "../interface"
interface Props {
    value: string
}

interface State {
    searchTerm: string,
    dropdownItems: GeocodeFeature[],
    activeGeocode: GeocodeFeature | undefined,
    dateRange: [Date, Date],
}

const state = reactive<State>({
    searchTerm: '',
    dropdownItems: [],
    activeGeocode: undefined,
    dateRange: [new Date(), new Date()],
})

const token = inject('spotifyToken');

watch(
    () => state.dateRange,
    (s, prevState) => {
        if (!!state.activeGeocode) {
            emitGeoSelect()
        }
    },
    { deep: true }
)

function isToday(someDate: Date): boolean {
    const today = new Date()
    return someDate.getDate() == today.getDate() &&
        someDate.getMonth() == today.getMonth() &&
        someDate.getFullYear() == today.getFullYear()
}
const singleDayPicked = computed(() => moment(state.dateRange[0]).isSame(moment(state.dateRange[1])))
const todayPicked = computed(() => singleDayPicked && isToday(state.dateRange[0]))
const singleDayText = computed(() => {
    return todayPicked ? 'Today' : moment(state.dateRange[0]).format("MMM Do")
})
const multiDayText = computed(() => [moment(state.dateRange[0]).format("MMM Do"), moment(state.dateRange[1]).format("MMM Do")])
const emit = defineEmits<{
    (e: 'geocode', item: GeocodeFeature): void
    (e: 'artist', item: SpotifyArtist): void
}>()

const queryTypeahead = debounce(() => {
    // Spotify
    const spotifyCall = fetch(`https://api.spotify.com/v1/search?q=${state.searchTerm}&type=artist&limit=5`, {
        headers: {
            'Authorization': `Bearer ${token.value}`
        }
    }).then(res => res.json())

    const key = import.meta.env.VITE_MB_KEY
    const geocodeCall = fetch(
        `https://api.mapbox.com/geocoding/v5/mapbox.places/${state.searchTerm}.json?types=place&access_token=${key}`
    ).then(response => response.json());
    const promises: Promise<any>[] = [spotifyCall, geocodeCall];
    Promise.all(promises).then(values => {
        console.log(values)
        state.dropdownItems = [...values[0].artists.items, ...values[1].features]

    });
}, 300);

function handleUpdate(query: string) {
    state.searchTerm = query;
    queryTypeahead()
}

function handleSelect(item: GeocodeFeature | SpotifyArtist) {
    if (item.type === 'artist') {
        state.searchTerm = item.name;
        emitArtistSelect(item)
    } else {
        state.activeGeocode = item;
        state.searchTerm = item.place_name;
        emitGeoSelect()
    }
    state.dropdownItems = [];
}

function emitGeoSelect() {
    emit('geocode', { geocode: state.activeGeocode, dateRange: state.dateRange });
}
function emitArtistSelect(item: SpotifyArtist) {
    emit('artist', item);
}
</script>

<template>
    <div class="search">
        <div class="search-wrapper">
            <div class="search_dropdown">
                <Search class="search-bar" :value="state.searchTerm" @update="handleUpdate" />
                <Datepicker
                    v-model="state.dateRange"
                    class="datepicker"
                    :dark="true"
                    :enable-time-picker="false"
                    :range="true"
                    :auto-apply="true"
                >
                    <template #trigger>
                        <div class="day-select">
                            <div class="first-day" v-if="singleDayPicked">{{ singleDayText }}</div>
                            <div v-else>{{ multiDayText[0] }} - {{ multiDayText[1] }}</div>
                        </div>
                    </template>
                </Datepicker>
            </div>
            <Dropdown
                v-show="!!state.dropdownItems.length"
                :items="state.dropdownItems"
                @select="handleSelect"
            />
        </div>
    </div>
</template>

<style scoped lang="scss">
.search {
    margin: 8px 0px 8px 8px;
    position: sticky;
    left: 0px;
    top: 0px;
    z-index: 15;
    transform: translateX(0px);
    transition: -webkit-transform 0.2s cubic-bezier(0, 0, 0.2, 1) 0s, transform,
        visibility, opacity;
    width: 100%;
}

.search-bar {
    position: relative;
    background: #fff;
    border-radius: 8px;
    box-sizing: border-box;
    width: 250px;
    height: 48px;
    border-bottom: 1px solid transparent;
    padding-left: 12px;
    -webkit-transition-property: background, box-shadow;
    transition-property: background, box-shadow;
    -webkit-transition-duration: 0.3s;
    transition-duration: 0.3s;
}

.search_dropdown {
    display: block;
}

.datepicker {
    cursor: pointer;
    padding: 12px 15px;
    position: absolute;
    right: 0;
    top: 0;
    display: block;
}
.search_bar {
    position: absolute;
    z-index: 4;
    left: 0;
    top: 0;
    box-shadow: 0 0 2px rgb(0 0 0 / 20%), 0 -1px 0 rgb(0 0 0 / 2%);
    border-radius: 8px 8px 0 0;
    border-bottom: 1px solid #dadce0;
    position: relative;
    background: #fff;
    box-sizing: border-box;
    width: 392px;
    height: 48px;
    border-bottom: 1px solid transparent;
    padding: 12px 104px 11px 64px;
    transition-property: background, box-shadow;
    transition-duration: 0.3s;
}
:global(.dp__theme_dark) {
    --dp-background-color: #162131;
    --dp-text-color: #dee5e5;
    --dp-hover-color: #556577;
    --dp-primary-color: #d7335c;
    --dp-primary-text-color: #dee5e5;
    --dp-secondary-color: #a9a9a9;
    --dp-border-color: #2d2d2d;
    --dp-border-color-hover: #aaaeb7;
    --dp-disabled-color: #737373;
    --dp-scroll-bar-background: #212121;
    --dp-scroll-bar-color: #484848;
    --dp-success-color: #d7335c;
    --dp-icon-color: #959595;
    --dp-danger-color: #e53935;
}
.day-select {
    background: var(--button-color);
    color: var(--dynamic-subtitle-color);
    border: 3px solid var(--dynamic-subtitle-color);
    border-radius: 26px;
    box-sizing: border-box;
    padding: 5px 15px 7px;
    font-size: 14px;
    cursor: pointer;
}

.day-select:hover {
    color: var(--button-hover-color);
    border: 3px solid var(--button-hover-color);
}

.search-wrapper {
    text-align: left;
}
</style>