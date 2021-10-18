<script setup lang="ts">
import { GeocodeFeature, SpotifyArtist } from "../interface";
import 'flag-icon-css/css/flag-icon.css'

interface Props {
    items: Array<GeocodeFeature | SpotifyArtist>,
}

const props = withDefaults(defineProps<Props>(), {
    items: [],
})
const emit = defineEmits<{
    (e: 'select', item: GeocodeFeature): void
}>()


function formatPlaceName(item: GeocodeFeature): string {
    // [county, state, country]
    const country = item.context.filter(context => context.id.startsWith('country'))[0];
    const region = item.context.filter(context => context.id.startsWith('region'))[0];
    return ['us', 'ca'].includes(country.short_code) ? `${item.text}, ${region.text}` : item.text;
}

function getPlaceCountry(item: GeocodeFeature): string {
    const country = item.context.filter(context => context.id.startsWith('country'))[0];
    return `flag-icon-${country.short_code}`;
}

function emitSelect(item: GeocodeFeature) {
    emit('select', item);
}
</script>

<template>
    <div class="dropdown">
        <div class="row" v-for="item in items" :key="item.id" @click="emitSelect(item)">
            <div v-if="item.popularity">
                <img class="artist-img" :src="item.images[0].url" />
                {{ item.name }}
            </div>
            <div v-else>
                <span :class="[getPlaceCountry(item), 'flag-icon']" />
                {{ formatPlaceName(item) }}
            </div>
        </div>
    </div>
</template>

<style scoped>
.dropdown {
    width: 90%;
    height: min-content;
    color: #333;
    margin: 6px auto;
    border-radius: 6px;
    max-height: 600px;
    font-family: Helvetica, sans-serif;
    font-size: 14px;
    background: hsl(0deg 0% 100% / 75%);
}
.row {
    padding: 6px;
    cursor: pointer;
}

.row:hover {
    background: rgb(32 32 32 / 76%);
    color: white;
}

.artist-img {
    height: 50px;
}
</style>