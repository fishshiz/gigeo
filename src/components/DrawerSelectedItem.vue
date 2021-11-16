<script setup lang="ts">
import { GeocodeFeature, TMClassification, TMEvent, Coordinates } from "../interface";
import { computed, reactive } from 'vue';
import moment from 'moment'
interface Props {
    event: TMEvent,
}
interface State {
    attractionHoverIdx: number
}
const state: State = reactive({
    attractionHoverIdx: -1
})
const props = defineProps<Props>();

const emit = defineEmits<{
    (e: 'back'): void,
    (e: 'artist-select', artist: string): void
}>()

function emitBack() {
    emit('back');
}
const eventLocation = computed(() => {
    const venues = props.event._embedded.venues;
    if (venues.length) {
        console.log('yup', venues)
        const venue = venues[0];
        console.log('y', venue, venue.name, venue.city.name, venue.state.name)
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

function emitArtist(name: string): void {
    emit('artist-select', name);
}

const classifications = computed(() => {
    const c: TMClassification = props.event.classifications.length ? props.event.classifications[0] : {}
    console.log(c);
    return {
        genre: c.genre.name,
        subGenre: c.subGenre.name,
        segment: c.segment.name,
        type: c.type ? c.type.name : '',
        subType: c.subType ? c.subType.name : '',
    }
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
                <div class="label">Featuring</div>
                <div class="attraction-wrapper">
                    <div
                        v-for="(attraction, idx) in attractions"
                        :key="attraction.id"
                        @mouseover="state.attractionHoverIdx = idx"
                        @mouseleave="state.attractionHoverIdx = -1"
                        :class="['attraction', { ['hover-attr']: state.attractionHoverIdx === idx }]"
                    >
                        <div class="circular--landscape" @click="emitArtist(attraction.name)">
                            <img class="attr-img" :src="attraction.images[0].url" />
                        </div>
                        <span class="attr-name">{{ attraction.name }}</span>
                    </div>
                </div>
            </div>
            <div class="label">Tags</div>
            <div class="classifications">
                <div class="tag">{{ classifications.genre }}</div>
                <div class="tag">{{ classifications.subGenre }}</div>
                <div class="tag">{{ classifications.segment }}</div>
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
    overflow: scroll;
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

.label {
    font-size: 18px;
    color: var(--dynamic-title-color);
    margin: 16px 0 12px 0;
}

.attraction {
    display: flex;
    flex-direction: column;
    align-items: center;
    font-size: 1em;
    color: var(--dynamic-subtitle-color);
}

.hover-attr {
    transform: scale(1.1);
    cursor: pointer;
}

.hover-attr.circular--landscape {
    filter: blur(1px);
}
.attraction-wrapper {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    margin-bottom: 12px;
}

.circular--landscape {
    display: inline-block;
    position: relative;
    width: 80px;
    height: 80px;
    overflow: hidden;
    border-radius: 50%;
}

.circular--landscape img {
    width: auto;
    height: 100%;
    margin-left: -40%;
}

.classifications {
    background-image: linear-gradient(
        var(--background-highlight-color),
        var(--button-color)
    );
    background: linear-gravar(--background-highlight-color);
    color: var(--dynamic-title-color);
    height: 60px;
    display: flex;
    justify-content: space-around;
    align-items: center;
    margin: 12px 0;
    border-radius: 6px;
}

.tag {
    border: 1px solid var(--dynamic-subtitle-color);
    padding: 6px 12px;
    border-radius: 6px;
    color: var(--dynamic-subtitle-color);
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

.attr-name {
    font-size: 12px;
    padding: 6px;
    color: var(--dynamic-subtitle-color);
}
</style>