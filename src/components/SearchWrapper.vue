<script setup lang="ts">
import Search from './Search.vue'
import Dropdown from './Dropdown.vue'
import debounce from 'lodash.debounce';
import moment from 'moment';
import Datepicker from 'vue3-date-time-picker';
import 'vue3-date-time-picker/dist/main.css'
import { reactive, ref, computed, defineExpose, onMounted, nextTick, watch } from 'vue'
import { GeocodeResponse, GeocodeFeature } from "../interface"
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

watch(
    () => state,
    (state, prevState) => {
        if (!!state.activeGeocode) {
            emitSelect()
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
    (e: 'select', item: GeocodeFeature): void
}>()

const queryTypeahead = debounce(() => {
    const key = import.meta.env.VITE_MB_KEY
    fetch(
        `https://api.mapbox.com/geocoding/v5/mapbox.places/${state.searchTerm}.json?types=place&access_token=${key}`
    ).then(response => response.json()).then(res => {
        state.dropdownItems = res.features;
    });
}, 300);

function handleUpdate(query: string) {
    state.searchTerm = query;
    queryTypeahead()
}

function handleSelect(item: GeocodeFeature) {
    state.activeGeocode = item;
    state.dropdownItems = [];
    state.searchTerm = item.place_name;
    emitSelect()
}

function emitSelect() {
    emit('select', { geocode: state.activeGeocode, dateRange: state.dateRange });
}
</script>

<template>
    <div class="search">
        <div>
            <Search :value="state.searchTerm" @update="handleUpdate" />
            <Dropdown :items="state.dropdownItems" @select="handleSelect" />
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
                    <div v-else>{{ multiDayText[0] }} - {{ multiDayText[1] }}</div>
                </div>
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

.first-day {
}

.day-select {
    background: rgb(27 38 55 / 51%);
    color: #dee5e5;
    border: 3px solid #dee5e5;
    border-radius: 26px;
    box-sizing: border-box;
    padding: 5px 15px 7px;
    font-size: 14px;
    cursor: pointer;
}

.day-select:hover {
    color: #9dc5bb;

    border: 3px solid #9dc5bb;
}
</style>