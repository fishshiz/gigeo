<script setup lang="ts">
import Search from './Search.vue'
import Dropdown from './Dropdown.vue'
import debounce from 'lodash.debounce';
import moment from 'moment';
import { reactive, onMounted } from 'vue'
import { GeocodeResponse, GeocodeFeature } from "../interface"
interface Props {
    value: string
}

const state = reactive({
    searchTerm: '',
    dropdownItems: [],
    isArtistSearch: false,
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
    emit('select', item);
}
</script>

<template>
    <div class="search">
        <Search :value="state.searchTerm" @update="handleUpdate" />
        <Dropdown :items="state.dropdownItems" @select="emitSelect" />
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
}
</style>