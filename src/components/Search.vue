<script setup lang="ts">
interface Props {
    value: string
}

const props = withDefaults(defineProps<Props>(), {
    value: ''
})

const emit = defineEmits<{
    (e: 'update', value: string): void
}>()

function updateValue(event: Event) {
    const input = event.target as HTMLInputElement;
    emit('update', input.value);
}
</script>

<template>
    <input type="text" placeholder="Search" :value="value" @input="updateValue" />
</template>

<style scoped>
input {
    width: 100%;
}
input {
    background: none;
    border: 3px solid #fff;
    border-radius: 26px;
    box-sizing: border-box;
    padding: 5px 15px 7px;
    font-size: 14px;
    color: #fff;
    z-index: 2;
    position: relative;
}
input:focus {
    outline: none;
}
input::placeholder {
    /* Chrome, Firefox, Opera, Safari 10.1+ */
    color: #fff;
    opacity: 1; /* Firefox */
}
</style>