<script lang="js">
export default {
  name: 'UiPagination',
  props: {
    page: {
      type: Number,
      required: true,
    },
    pageCount: {
      type: Number,
      required: true,
    },
    previousLabel: {
      type: String,
      required: true,
    },
    nextLabel: {
      type: String,
      required: true,
    },
    summaryLabel: {
      type: String,
      required: true,
    },
    metaLabel: {
      type: String,
      default: '',
    },
    pageInfoLabel: {
      type: String,
      default: '',
    },
    rootTestId: {
      type: String,
      default: 'ui-pagination',
    },
    previousButtonTestId: {
      type: String,
      default: 'ui-pagination-prev',
    },
    nextButtonTestId: {
      type: String,
      default: 'ui-pagination-next',
    },
    summaryTestId: {
      type: String,
      default: 'ui-pagination-summary',
    },
    pageInfoTestId: {
      type: String,
      default: 'ui-pagination-page-info',
    },
  },
  emits: ['update:page'],
  computed: {
    safePageCount() {
      return Math.max(1, this.pageCount)
    },
    safePage() {
      return Math.min(this.safePageCount, Math.max(1, this.page))
    },
    canGoPrevious() {
      return this.safePage > 1
    },
    canGoNext() {
      return this.safePage < this.safePageCount
    },
    resolvedPageInfoLabel() {
      return this.pageInfoLabel || `${this.safePage} / ${this.safePageCount}`
    },
  },
  methods: {
    goPrevious() {
      if (!this.canGoPrevious) {
        return
      }

      this.$emit('update:page', this.safePage - 1)
    },
    goNext() {
      if (!this.canGoNext) {
        return
      }

      this.$emit('update:page', this.safePage + 1)
    },
  },
}
</script>

<template>
  <div class="ui-pagination" :data-testid="rootTestId">
    <div class="meta">
      <span v-if="metaLabel" class="meta-label">{{ metaLabel }}</span>
      <span class="summary" :data-testid="summaryTestId">{{ summaryLabel }}</span>
    </div>

    <div class="actions">
      <button
        type="button"
        class="ghost-button"
        :data-testid="previousButtonTestId"
        :disabled="!canGoPrevious"
        @click="goPrevious"
      >
        <slot name="previousIcon" />
        <span>{{ previousLabel }}</span>
      </button>

      <span
        v-if="pageInfoLabel"
        class="page-info"
        :data-testid="pageInfoTestId"
      >
        {{ resolvedPageInfoLabel }}
      </span>

      <button
        type="button"
        class="ghost-button"
        :data-testid="nextButtonTestId"
        :disabled="!canGoNext"
        @click="goNext"
      >
        <span>{{ nextLabel }}</span>
        <slot name="nextIcon" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.ui-pagination,
.meta,
.actions {
  display: flex;
  align-items: center;
}

.ui-pagination {
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
  margin-top: 0.1rem;
  padding-top: 0.25rem;
}

.meta,
.actions {
  gap: 0.65rem;
  flex-wrap: wrap;
}

.summary,
.meta-label,
.page-info {
  color: var(--text-muted);
  font-size: 0.82rem;
}

@media (max-width: 880px) {
  .ui-pagination,
  .meta,
  .actions {
    width: 100%;
  }

  .ui-pagination,
  .actions {
    justify-content: space-between;
  }
}
</style>
