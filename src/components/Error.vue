<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup
import { watch } from 'vue';
interface Props {
    value: boolean,
}

const props = defineProps<Props>()
const emit = defineEmits<{
    (e: "update:value", val: boolean): void
}>()

watch(() => props.value, (val) => {
    console.log('ji')
    if (val) {
        setTimeout(() => {
            emit('update:value', false)
        }, 500)
    }
})
</script>

<template>
    <div class="error-wrapper" v-show="props.value">
        <span class="material-icons error">&#xe002;</span>
        <slot class="slot"></slot>
    </div>
</template>

<style lang="scss">
.error-wrapper {
    position: absolute;
    bottom: 16px;
    background: var(--dynamic-subtitle-color);
    right: 16px;
    width: 400px;
    height: 100px;
    padding: 16px;
    border-radius: 12px;
}

.material-icons.error {
    cursor: pointer;
    color: rgba(255, 255, 255, 1);
    color: #f6a38e;
    text-shadow: 0 0 1px #f9b4a3;
}

.slot {
    color: var(--dynamic-border-color);
    font-size: 30px;
}
</style>
