export default defineNuxtPlugin((nuxtApp) => {
  const observerCallback = (entries: IntersectionObserverEntry[]) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) {
        entry.target.classList.add('is-visible')
      }
    })
  }

  nuxtApp.vueApp.directive('reveal', {
    mounted(el) {
      if (process.client) {
        el.classList.add('reveal-on-scroll')
        const observer = new IntersectionObserver(observerCallback, {
          threshold: 0.1,
          rootMargin: '0px 0px -50px 0px'
        })
        observer.observe(el)
        // Store observer on element to unobserve later
        el._revealObserver = observer
      }
    },
    unmounted(el) {
      if (process.client && el._revealObserver) {
        el._revealObserver.unobserve(el)
      }
    },
    // This ensures the server-side renderer knows how to handle the directive
    getSSRProps() {
      return {}
    }
  })
})
