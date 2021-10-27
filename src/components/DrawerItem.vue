<script setup lang="ts">
import { GeocodeFeature, TMImage, TMVenue } from "../interface";
import moment from "moment";

defineProps<{
    title: string,
    day: string,
    time: string,
    images: TMImage[],
    venue: TMVenue,
    ticketLink: string,
    hover: boolean,
}>()
const emit = defineEmits<{
    (e: 'select', item: GeocodeFeature): void,
}>()

function formatDate(date: string) {
    return moment(date).format('MMM Do h:mm a')
}

function truncate(str: string, length: number = 16): string {
    return str.length <= length ? str : str.slice(0, length - 3).concat('...')
}

</script>

<template>
    <div :class="['item-wrapper', { ['hover']: hover }]">
        <div class="item">
            <div class="top-row">
                <div class="header">
                    <div class="title">{{ truncate(title, 50) }}</div>
                    <div class="event-info">
                        <div>{{ truncate(venue.name, 30) }}</div>
                        <div>{{ formatDate(day) }}</div>
                    </div>
                </div>
                <div class="image-container">
                    <div class="image-filter">
                        <img
                            decoding="async"
                            class="item-image"
                            :src="images[0].url"
                            aria-hidden="true"
                        />
                    </div>
                </div>
            </div>
            <a :href="ticketLink" target="_blank" class="tickets">Tickets</a>
        </div>
    </div>
</template>

<style scoped lang="scss">
.item-wrapper {
    position: relative;
    z-index: 2;
    padding: 8px 0 16px 0;
    height: 120px;
    border-bottom: 1 px solid var(--dynamic-border-color);
}

.hover {
    background: var(--background-highlight-color);
    box-shadow: 0 2px 4px rgb(0 0 0 / 20%), 0 -1px 0 rgb(0 0 0 / 2%);
}

.image-container {
    display: inline-block;
    margin: 8px 0 8px 2px;
    -webkit-flex: none;
    -webkit-box-flex: 0;
    flex: none;
    position: relative;
    border-radius: 8px;
    width: 84px;
    height: 84px;
    margin: 0px;
}

.image-filter {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    bottom: 0;
    overflow: hidden;
    transition-property: opacity, -webkit-filter, filter;
    -webkit-transition-duration: 0.15s, 0.3s, 0.3s;
    transition-duration: 0.15s, 0.3s, 0.3s;
    -webkit-transition-timing-function: cubic-bezier(0.4, 0, 1, 1);
    transition-timing-function: cubic-bezier(0.4, 0, 1, 1);
    background-color: var(--dynamic-border-color);
    opacity: 0;
    -webkit-filter: saturate(0);
    filter: saturate(0);
    border-radius: 8px;
    opacity: 1;
    -webkit-filter: saturate(100%);
    filter: saturate(100%);
    min-width: 84px;
    min-height: 84px;
}

.title {
    line-height: 20px;
    color: var(--dynamic-title-color);
    white-space: normal;
    text-align: left;
    margin-bottom: 2px;
    font-weight: 500;
}

.event-info {
    font-size: 14px;
    color: var(--dynamic-subtitle-color);
}
.item {
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 8px 18px;
    align-items: flex-start;
}

.item-image {
    position: absolute;
    top: 50%;
    left: 50%;
    height: 84px;
    -webkit-transform: translateY(-50%) translateX(-50%);
    transform: translateY(-50%) translateX(-50%);
}

.top-row {
    text-decoration: none;
    height: 100%;
    width: 100%;
    color: var(--dynamic-title-color);
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    font-family: Roboto, Arial, sans-serif;
    text-align: left;
    box-sizing: border-box;
}

.header {
    max-width: 200px;
}

.tickets {
    border: 1 px solid var(--button-hover-color);
    border-radius: 17px;
    display: block;
    color: var(--button-color);
    font-size: 14px;
    padding: 8px 16px;
    text-align: center;
    text-decoration: none;
    cursor: pointer;
    font-family: Roboto, Arial, sans-serif;
}

.tickets:hover {
    color: var(--button-hover-color);
    background-color: #d2e3fc;
    border-color: #aecbfa;
}
</style>