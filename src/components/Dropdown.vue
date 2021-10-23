<script setup lang="ts">
import type { GeocodeFeature, SpotifyArtist } from "../interface";
import 'flag-icon-css/css/flag-icon.css'

interface Props {
    items: Array<GeocodeFeature | SpotifyArtist>,
}

const props = defineProps<Props>();
const emit = defineEmits<{
    (e: 'select', item: GeocodeFeature): void
}>()


function formatPlaceName(item: GeocodeFeature): string {
    // [county, state, country]
    const country = item.context.filter(context => context.id.startsWith('country'))[0];
    const region = item.context.filter(context => context.id.startsWith('region'))[0];
    return region ? `${item.text}, ${region.text}` : item.text;
}

function getPlaceCountry(item: GeocodeFeature): string {
    const country = item.context.filter(context => context.id.startsWith('country'))[0];
    return `flag-icon-${country.short_code}`;
}

function artistImage(item: SpotifyArtist): string {
    return item.images.length ? item.images[0].url : '../images/artist.png';
}

function emitSelect(item: GeocodeFeature | SpotifyArtist) {
    emit('select', (item as any));
}
</script>

<template>
    <div class="dropdown">
        <div class="row" v-for="item in props.items" :key="item.id" @click="emitSelect(item)">
            <div v-if="item.type === 'artist'">
                <img class="artist-img" :src="artistImage((item as SpotifyArtist))" />
                {{ (item as SpotifyArtist).name }}
            </div>
            <div v-else>
                <span :class="[getPlaceCountry((item as GeocodeFeature)), 'flag-icon', 'flag']" />
                {{ formatPlaceName((item as GeocodeFeature)) }}
            </div>
        </div>
    </div>
</template>

<style scoped>
.dropdown {
    position: relative;
    text-align: left;
    min-width: 224px;
    background-color: #fff;
    border-radius: 1.1rem;
    box-shadow: 0 2px 4px rgb(0 0 0 / 20%);
    font-size: 15px;
    top: 4px;
    overflow: hidden;
    width: 260px;
    z-index: 100;
    padding: 8px 0;
}
.row {
    padding: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
}

.row:hover {
    background: rgb(32 32 32 / 76%);
    color: white;
}

.artist-img,
.flag {
    float: left;
    height: 20px;
    width: 20px;
    margin: 2px 26px 0 18px;
}
</style>