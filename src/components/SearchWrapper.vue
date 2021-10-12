<script setup lang="ts">
import Search from './Search.vue'
import Dropdown from './Dropdown.vue'
import debounce from 'lodash.debounce';
import moment from 'moment';
import Datepicker from 'vue3-date-time-picker';
import 'vue3-date-time-picker/dist/main.css'
import { reactive, ref, computed, defineExpose, onMounted, nextTick } from 'vue'
import { GeocodeResponse, GeocodeFeature } from "../interface"
interface Props {
    value: string
}

const state = reactive({
    searchTerm: '',
    dropdownItems: [],
    isArtistSearch: false,
    dateRange: [new Date(), new Date()],
})

function isToday(someDate: Date): boolean {
    const today = new Date()
    return someDate.getDate() == today.getDate() &&
        someDate.getMonth() == today.getMonth() &&
        someDate.getFullYear() == today.getFullYear()
}
const singleDayPicked = computed(() => moment(state.dateRange[0]).isSame(moment(state.dateRange[1])))
const todayPicked = computed(() => singleDayPicked && isToday(state.dateRange[0]))
const singleDayText = computed(() => {
    return !todayPicked ? 'Today' : moment(state.dateRange[0]).format("MMM Do")
})
const emit = defineEmits<{
    (e: 'select', item: GeocodeFeature): void
}>()

const queryTypeahead = debounce(() => {
    const key = import.meta.env.VITE_MB_KEY
    fetch(
        `https://api.mapbox.com/geocoding/v5/mapbox.places/${state.searchTerm}.json?country=us&types=place&access_token=${key}`
    ).then(response => response.json()).then(res => {
        state.dropdownItems = res.features;
    });
}, 300);

function handleUpdate(query: string) {
    state.searchTerm = query;
    queryTypeahead()
}

function emitSelect(item: GeocodeFeature) {
    state.dropdownItems = [];
    state.searchTerm = item.place_name;
    emit('select', item);
}
</script>

<template>
    <div class="search">
        <div>
            <Search :value="state.searchTerm" @update="handleUpdate" />
            <Dropdown :items="state.dropdownItems" @select="emitSelect" />
        </div>
        <Datepicker
            v-model="state.dateRange"
            :dark="true"
            :enable-time-picker="false"
            :range="true"
        >
            <template #trigger>
                <div v-if="singleDayPicked">
                    <div class="first-day">{{ singleDayText }}</div>
                    <div class="arrow-right" />
                </div>
                <div v-else>{{ state.dateRange[0] }} - {{ state.dateRange[1] }}</div>
            </template>
        </Datepicker>
    </div>
</template>

<style scoped>
.search {
    background: rgb(27 38 55 / 51%);
    margin: 0 auto;
    width: 300px;
    position: absolute;
    left: calc(50% - 120px);
    top: 40px;
    display: flex;
}
.dp__theme_dark {
    --dp-background-color: RGB(26, 37, 56);
    --dp-text-color: #ffffff;
    --dp-hover-color: #484848;
    --dp-primary-color: #005cb2;
    --dp-primary-text-color: #ffffff;
    --dp-secondary-color: #a9a9a9;
    --dp-border-color: #2d2d2d;
    --dp-border-color-hover: #aaaeb7;
    --dp-disabled-color: #737373;
    --dp-scroll-bar-background: #212121;
    --dp-scroll-bar-color: #484848;
    --dp-success-color: #00701a;
    --dp-icon-color: #959595;
    --dp-danger-color: #e53935;
}
.arrow-right {
    width: 0;
    height: 0;
    border-top: 15px solid transparent;
    border-bottom: 15px solid transparent;
    border-left: 15px solid #fff;
    display: inline;
    position: absolute;
}

.first-day {
    background: #fff;
    display: inline-block;
    height: 30px;
}
</style>