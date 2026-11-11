import { GlobalWindow } from 'happy-dom'

const window = new GlobalWindow({
  url: 'http://localhost:3000',
  width: 1280,
  height: 800,
})

// @ts-ignore
globalThis.window = window
// @ts-ignore
globalThis.document = window.document
// @ts-ignore
globalThis.HTMLElement = window.HTMLElement
// @ts-ignore
globalThis.HTMLButtonElement = window.HTMLButtonElement
// @ts-ignore
globalThis.HTMLInputElement = window.HTMLInputElement
// @ts-ignore
globalThis.HTMLCanvasElement = window.HTMLCanvasElement
// @ts-ignore
globalThis.getComputedStyle = window.getComputedStyle.bind(window)
