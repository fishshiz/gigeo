<template>
    <div class="item">
        <i class="loader --2"></i>
    </div>
</template>

<style scoped lang="scss">
.loader {
    --color: var(--button-color);
    --size-mid: 6vmin;
    --size-dot: 1.5vmin;
    --size-bar: 0.4vmin;
    --size-square: 3vmin;

    display: block;
    position: relative;
    width: 50%;
    display: grid;
    place-items: center;
}

.loader::before,
.loader::after {
    content: "";
    box-sizing: border-box;
    position: absolute;
}

/**
	loader --2
**/
.loader.--2::before,
.loader.--2::after {
    width: var(--size-dot);
    height: var(--size-dot);
    background-color: var(--color);
    border-radius: 50%;
    opacity: 0;
    animation: loader-2 0.8s cubic-bezier(0.2, 0.32, 0, 0.87) infinite;
}

.loader.--2::after {
    animation-delay: 0.3s;
}

@keyframes loader-2 {
    0%,
    80%,
    100% {
        opacity: 0;
    }

    33% {
        opacity: 1;
    }

    0%,
    100% {
        transform: translateX(-4vmin);
    }

    90% {
        transform: translateX(4vmin);
    }
}
.item {
    background: rgba(255, 255, 255, 0.1);
    display: grid;
    place-items: center;
    border-radius: 4px;
    transition: opacity 0.4s ease;
}
</style>