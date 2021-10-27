<script setup lang="ts">
import { GeocodeFeature, SpotifyArtist, TMEvent, Coordinates } from "../interface";
import { computed } from 'vue';
import moment from 'moment'
interface Props {
    event: TMEvent,
}
const props = defineProps<Props>();

const emit = defineEmits<{
    (e: 'back'): void,
}>()

function emitBack() {
    emit('back');
}
const eventLocation = computed(() => {
    const venues = props.event._embedded.venues;
    if (venues.length) {
        console.log('yup', venues)
        const venue = venues[0];
        const venueName = venue.name;
        const city = venue.city.name;
        const state = venue.state.name;
        return `${venueName} • ${city}, ${state}`;
    }
    return venues.length ? venues[0].name : 'N/A';
})
const priceRange = computed(() => {
    const price = props.event.priceRanges;
    if (price) {

        // Create our number formatter.
        const formatter = new Intl.NumberFormat('en-US', {
            style: 'currency',
            currency: 'USD',

            // These options are needed to round to whole numbers if that's what you want.
            //minimumFractionDigits: 0, // (this suffices for whole numbers, but will print 2500.10 as $2,500.1)
            //maximumFractionDigits: 0, // (causes 2500.99 to be printed as $2,501)
        });

        return formatter.format(price[0].min); /* $2,500.00 */
    }
    return 'No price available'
})

const attractions = computed(() => {
    return !!props.event._embedded.attractions ? props.event._embedded.attractions : []
})

const date = computed(() => {
    const startDate = props.event.dates.start.dateTime;
    // Create our number formatter.
    return moment(startDate).format('MMM Do h:mm a')
})
</script>

<template>
    <div class="wrapper">
        <div class="banner">
            <img class="selected-image" :src="event.images[0].url" />
            <span class="material-icons back" @click="emitBack">&#xe5cb;</span>
            <h1 class="selected-title">{{ event.name }}</h1>
        </div>

        <div class="content">
            <div class="sub-banner">
                <div>{{ date }}</div>
                <div>{{ eventLocation }}</div>
                <div>Starts at {{ priceRange }}</div>
            </div>

            <div v-if="attractions.length">
                <div v-for="attraction in attractions" :key="attraction.id">
                    <img class="attr-img" :src="attraction.images[0].url" />
                    {{ attraction.name }}
                </div>
            </div>

            <div class="note" v-if="event.pleaseNote">
                <span class="material-icons note-info">&#xe88e;</span>
                {{ event.pleaseNote }}
            </div>
        </div>
    </div>
</template>

<style scoped lang="scss">
.wrapper {
    overflow: visible;
}

.banner {
    position: relative;
}

.sub-banner {
    font-size: 0.875rem;
    line-height: 1.25rem;
    color: var(--dynamic-subtitle-color);
    margin-bottom: 8px;
}

.selected-image {
    width: 408px;
}

.attr-img {
    width: 108px;
}
.selected-title {
    color: var(--dynamic-title-color);
    font-size: 1.6rem;
    position: absolute;
    bottom: 8px;
    left: 8px;
    background: var(--button-color);
    padding: 2px 8px;
    opacity: 0.85;
}
.material-icons.back {
    color: var(--dynamic-title-color);
    font-size: 1.6rem;
    position: absolute;
    top: 8px;
    left: 8px;
    background: var(--button-color);
    padding: 2px;
    opacity: 0.85;
    cursor: pointer;
}
.note {
    background-color: var(--dynamic-subtitle-color);
    padding: 8px;
    border-radius: 6px;
    color: var(--app-background-color);
    max-height: 150px;
    text-align: justify;
    text-indent: 4px;
    line-height: 1.3;
    overflow: scroll;
}

.material-icons.note-info {
    float: none;
}

.content {
    margin: 0 12px;
}
</style>