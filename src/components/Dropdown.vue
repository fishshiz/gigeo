<script setup lang="ts">
import { GeocodeFeature } from "../interface";
import 'flag-icon-css/css/flag-icon.css'

interface Props {
    items: GeocodeFeature[],
}

const props = defineProps({
    items: Object as () => GeocodeFeature[],
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
    <div class="dropdown" v-show="!!(props.items as GeocodeFeature[]).length">
        <div class="suggestions">
            <div class="row" v-for="item in props.items" :key="item.id" @click="emitSelect(item)">
                <span :class="[getPlaceCountry(item), 'flag-icon']" />
                {{ formatPlaceName(item) }}
            </div>
        </div>
    </div>
</template>

<style scoped>
.dropdown {
    position: relative;
    text-align: left;
    min-width: 224px;
}
.row {
    padding: 6px;
    cursor: pointer;
}

.row:hover {
    background: rgb(32 32 32 / 76%);
    color: white;
}

.suggestions {
    background-color: #fff;
    border-radius: 0 0 8px 8px;
    box-shadow: 0 2px 4px rgb(0 0 0 / 20%);
    font-size: 15px;
    overflow: hidden;
    padding: 8px 0;
}
</style>