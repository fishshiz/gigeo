<script setup lang="ts">
import { GeocodeFeature, TMEvent, TMImage, TMVenue } from "../interface";
import 'flag-icon-css/css/flag-icon.css'
import DrawerItem from './DrawerItem.vue';
import moment from "moment";

const props = defineProps<{
    title: string,
    day: string,
    time: string,
    images: TMImage[],
    venue: TMVenue
}>()
const emit = defineEmits<{
    (e: 'select', item: GeocodeFeature): void
}>()

function formatDate(date: string) {
    return moment(date).format('MMM Do h:mm a')
}

</script>

<template>
    <div class="item-wrapper">
        <div class="item">
            <div class="header">
                <div class="title">{{ title }}</div>
                <div class="event-info">
                    <div>{{ venue.name }}</div>
                    <div>{{ formatDate(day) }}</div>
                </div>
            </div>
            <div class="image-container">
                <div class="image-filter">
                    <img
                        decoding="async"
                        :src="images[0].url"
                        style="position: absolute;
    top: 50%;
    left: 50%;
    height: 84px;
    -webkit-transform: translateY(-50%) translateX(-50%);
    transform: translateY(-50%) translateX(-50%);"
                        aria-hidden="true"
                    />
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.item-wrapper {
    position: relative;
    z-index: 2;
    pointer-events: none;
    padding-top: 8px;
    padding-bottom: 16px;
    height: 120px;
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
    background-color: #f1f3f4;
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
    color: #202124;
    white-space: normal;
    text-align: left;
    margin-bottom: 2px;
    font-weight: 500;
}

.event-info {
    font-size: 14px;
    color: #70757a;
}
.item {
    text-decoration: none;
    height: 100%;
    width: 100%;
    box-sizing: border-box;
    color: #000;
    cursor: pointer;
    display: flex;
    font-family: Roboto, Arial, sans-serif;
    padding: 0;
    text-align: left;
    width: 100%;
}
</style>