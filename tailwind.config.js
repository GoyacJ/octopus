/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['class', '[data-theme="dark"]'],
  content: [
    './apps/desktop/index.html',
    './apps/desktop/src/**/*.{vue,js,ts,jsx,tsx}',
    './packages/ui/src/**/*.{vue,js,ts,jsx,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        background: 'var(--bg-main)',
        foreground: 'var(--foreground)',
        surface: 'var(--bg-surface)',
        sidebar: 'var(--bg-sidebar)',
        subtle: 'var(--bg-subtle)',
        muted: 'var(--bg-muted)',
        card: 'var(--bg-card)',
        popover: 'var(--bg-popover)',
        accent: 'var(--bg-accent)',
        secondary: 'var(--bg-secondary)',
        glass: 'var(--bg-glass)',
        input: 'var(--border-input)',
        ring: 'var(--ring)',
        'border-subtle': 'var(--border-subtle)',
        'border-strong': 'var(--border-strong)',
        border: 'var(--border-subtle)', // Alias for default border
        primary: {
          DEFAULT: 'var(--brand-primary)',
          hover: 'var(--brand-primary-hover)',
          foreground: 'var(--text-on-brand)',
        },
        destructive: {
          DEFAULT: 'var(--destructive)',
          foreground: 'var(--destructive-foreground)',
        },
        text: {
          primary: 'var(--text-primary)',
          secondary: 'var(--text-secondary)',
          tertiary: 'var(--text-tertiary)',
        },
        status: {
          success: 'var(--status-success)',
          warning: 'var(--status-warning)',
          error: 'var(--status-error)',
          info: 'var(--status-info)',
        },
        'muted-foreground': 'var(--muted-foreground)',
        'popover-foreground': 'var(--popover-foreground)',
        'accent-foreground': 'var(--accent-foreground)',
        'secondary-foreground': 'var(--secondary-foreground)',
      },
      borderRadius: {
        xs: 'var(--radius-xs)',
        s: 'var(--radius-s)',
        m: 'var(--radius-m)',
        l: 'var(--radius-l)',
        xl: 'var(--radius-xl)',
        '2xl': 'var(--radius-2xl)',
        full: 'var(--radius-full)',
      },
      fontFamily: {
        sans: ['Inter', 'SF Pro Display', 'SF Pro Text', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'sans-serif'],
        mono: ['SF Mono', 'JetBrains Mono', 'Fira Code', 'monospace'],
      },
      boxShadow: {
        xs: 'var(--shadow-xs)',
        sm: 'var(--shadow-sm)',
        md: 'var(--shadow-md)',
        lg: 'var(--shadow-lg)',
        xl: 'var(--shadow-xl)',
      },
      transitionTimingFunction: {
        apple: 'var(--ease-apple)',
      },
      transitionDuration: {
        fast: 'var(--duration-fast)',
        normal: 'var(--duration-normal)',
        slow: 'var(--duration-slow)',
      },
    },
  },
  plugins: [],
}
