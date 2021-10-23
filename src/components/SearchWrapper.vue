<script setup lang="ts">
import Search from './Search.vue'
import Dropdown from './Dropdown.vue'
import debounce from "just-debounce-it";
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
const todayPicked = computed(() => {
    const compareDate = state.dateRange[0];
    const today = new Date()
    return singleDayPicked && compareDate.getDate() === today.getDate() &&
        compareDate.getMonth() === today.getMonth() &&
        compareDate.getFullYear() === today.getFullYear()
})
const tomorrowPicked = computed(() => {
    const compareDate = moment(state.dateRange[0]);
    const tomorrow = moment().add(1, 'days');
    return compareDate.isSame(tomorrow, "day");
})
const singleDayText = computed(() => {
    if (todayPicked.value) {
        return 'Today';
    }
    if (tomorrowPicked.value) {
        return 'Tomorrow'
    }
    return moment(state.dateRange[0]).format("MMM Do")
})
const multiDayDay = computed(() => {
    const startDay = ordinalSuffix(moment(state.dateRange[0]).date());
    const endDay = ordinalSuffix(moment(state.dateRange[1]).date());;
    return [startDay, endDay]
})
const multiDayMonth = computed(() => {
    const startMonth = moment(state.dateRange[0]).format("MMM");
    const endMonth = moment(state.dateRange[1]).format("MMM");
    return startMonth === endMonth ? [startMonth] : [startMonth, endMonth];
})
const emit = defineEmits<{
    (e: 'geocode', item: GeocodeFeature): void
    (e: 'artist', item: SpotifyArtist): void
}>()

function ordinalSuffix(i: number) {
    const j = i % 10,
        k = i % 100;
    if (j == 1 && k != 11) {
        return i + "st";
    }
    if (j == 2 && k != 12) {
        return i + "nd";
    }
    if (j == 3 && k != 13) {
        return i + "rd";
    }
    return i + "th";
}
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
        state.searchTerm = (item as SpotifyArtist).name;
        emitArtistSelect((item as SpotifyArtist))
    } else {
        state.activeGeocode = (item as GeocodeFeature);
        state.searchTerm = (item as GeocodeFeature).place_name;
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
                <Dropdown
                    v-show="!!state.dropdownItems.length"
                    :items="state.dropdownItems"
                    @select="handleSelect"
                />
            </div>
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
                        <div v-else class="multi-day">
                            <div class="months">
                                <div class="month" v-for="month in multiDayMonth">{{ month }}</div>
                            </div>
                            <div class="days">
                                <div class="day">{{ multiDayDay[0] }}</div>
                                <div class="day">{{ multiDayDay[1] }}</div>
                            </div>
                        </div>
                    </div>
                </template>
            </Datepicker>
        </div>
    </div>
</template>

<style scoped lang="scss">
.search {
    margin: 8px 0px 8px 16px;
    position: sticky;
    height: 80px;
    z-index: 100;
}

.search-bar {
    position: relative;
    background: #fff;
    border-radius: 1.1rem;
    box-sizing: border-box;
    width: 260px;
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
    padding: 0px 15px;
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
    box-sizing: border-box;
    font-size: 14px;
    cursor: pointer;
    width: 100px;
    height: 50px;
    background-color: var(--button-color);
    border-radius: 1.1rem;
    text-align: center;
    line-height: 44px;
    color: var(--dynamic-subtitle-color);
    -webkit-box-shadow: 0px 13px 74px -15px rgb(129 87 244);
    -moz-box-shadow: 0px 13px 74px -15px rgba(129, 87, 244, 1);
    box-shadow: 0px 13px 74px -15px rgb(129 87 244);
    border: 3px solid transparent;
}

.day-select:hover {
    color: var(--button-hover-color);
    border: 3px solid var(--button-hover-color);
}

.search-wrapper {
    text-align: left;
}
.search-wrapper {
    text-align: left;
}

.multi-day {
    line-height: 1;
    display: grid;
    height: 100%;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
}

.months,
.days {
    display: flex;
    justify-content: space-evenly;
    align-items: center;
    grid-column: span 2;
}

.month {
    font-weight: 600;
}

.day {
    padding: 0 6px;
    box-sizing: border-box;
}
</style>