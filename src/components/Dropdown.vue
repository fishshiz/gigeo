<script setup lang="ts">
import { GeocodeFeature } from "../interface";

interface Props {
    items: GeocodeFeature[],
}

const props = withDefaults(defineProps<Props>(), {
    items: [],
})
const emit = defineEmits<{
    (e: 'select', item: GeocodeFeature): void
}>()


function formatPlaceName(item: GeocodeFeature): string {
    // [county, state, country]
    const state = item.context[1];
    return `${item.text}, ${state.text}`;
}

function emitSelect(item: GeocodeFeature) {
    emit('select', item);
}
</script>

<template>
    <div class="dropdown">
        <div
            class="row"
            v-for="item in items"
            :key="item.id"
            @click="emitSelect(item)"
        >{{ formatPlaceName(item) }}</div>
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
</style>