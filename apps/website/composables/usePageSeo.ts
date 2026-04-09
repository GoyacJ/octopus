export function usePageSeo(pageKey: 'home' | 'product' | 'scenarios' | 'about' | 'bookDemo') {
  const route = useRoute()
  const { t } = useI18n()
  const config = useRuntimeConfig()

  const canonicalUrl = computed(() => new URL(route.path || '/', config.public.siteUrl).toString())
  const title = computed(() => `${t(`seo.${pageKey}.title`)} · Octopus`)
  const description = computed(() => t(`seo.${pageKey}.description`))
  const siteName = computed(() => t('site.name'))

  useSeoMeta({
    title,
    ogTitle: title,
    description,
    ogDescription: description,
    ogSiteName: siteName,
    ogUrl: canonicalUrl,
    ogType: 'website',
    ogImage: new URL('/brand/og-cover.png', config.public.siteUrl).toString(),
    twitterCard: 'summary_large_image',
    twitterTitle: title,
    twitterDescription: description,
    twitterImage: new URL('/brand/og-cover.png', config.public.siteUrl).toString(),
  })

  useHead(() => ({
    link: [
      {
        rel: 'canonical',
        href: canonicalUrl.value,
      },
    ],
  }))
}
