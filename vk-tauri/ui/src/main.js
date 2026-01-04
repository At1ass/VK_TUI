import './app.css'
import { mount } from 'svelte'
import App from './App.svelte'

// Wait for DOM to be ready
function initApp() {
  const target = document.getElementById('app')

  if (!target) {
    console.error('App target not found, retrying...')
    setTimeout(initApp, 100)
    return
  }

  try {
    const app = mount(App, { target })
    return app
  } catch (error) {
    console.error('Failed to mount app:', error)
    // Auto-reload on error (during development)
    if (import.meta.env.DEV) {
      console.log('Reloading in 500ms...')
      setTimeout(() => window.location.reload(), 500)
    }
  }
}

// Start when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp)
} else {
  initApp()
}

export default initApp
