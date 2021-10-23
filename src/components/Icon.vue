<script setup lang="ts">
interface Props {
    icon: string,
    size: number,
    color: string
}
const props = defineProps<Props>()
</script>

<template>
    <svg :width="`${props.size}px`" :height="`${props.size}px`">
        <use :href="`./src/icons/${props.icon}.svg#icon`" :fill="props.color" />
    </svg>
</template>