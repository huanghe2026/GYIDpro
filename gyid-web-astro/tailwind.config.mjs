/**
 * 主题与 diliy.cn 保持一致（配色 / 发光 / 字体 / 动效）。
 * 设计令牌来源：diliy.cn 的 tailwind.config.mjs。
 *
 * @type {import('tailwindcss').Config}
 */
export default {
  content: ['./src/**/*.{astro,html,js,jsx,md,mdx,ts,tsx,vue}'],
  theme: {
    extend: {
      colors: {
        // 全站统一为首页配色：翡翠绿主强调 + 紫罗兰/粉/琥珀点缀
        'cyber-blue': '#34d399', // emerald-400 主强调
        'cyber-pink': '#f472b6', // pink-400
        'cyber-purple': '#8b5cf6', // violet-500
        'cyber-green': '#10b981', // emerald-500
        'cyber-neon': '#34d399', // emerald-400
        'cyber-orange': '#f59e0b', // amber-500
        'cyber-yellow': '#fbbf24', // amber-400
        'cyber-teal': '#2dd4bf', // teal-400
        'cyber-magenta': '#a78bfa', // violet-400
        'cyber-cyan': '#6ee7b7', // emerald-300
        'cyber-red': '#fb7185', // rose-400
        // Geoyuan MOCK UI — 深蓝黑调层
        'gy-deepest': '#020617', // 页面背景
        'gy-panel': '#060c1a', // 卡片/面板深底
        'gy-pop': '#06131f', // 弹窗深底
        'gy-emerald': {
          50: '#ecfdf5',
          100: '#d1fae5',
          200: '#a7f3d0',
          300: '#6ee7b7',
          400: '#34d399',
          500: '#10b981',
          600: '#059669',
          700: '#047857',
        },
        'gy-violet': {
          400: '#a78bfa',
          500: '#8b5cf6',
          600: '#7c3aed',
          700: '#6d28d9',
        },
        'gy-amber': {
          300: '#fcd34d',
          400: '#fbbf24',
          500: '#f59e0b',
        },
        'gy-pink': {
          400: '#f472b6',
          500: '#ec4899',
        },
      },
      spacing: {
        'gy-card': '22rem',
        'gy-feat': '3.5rem',
      },
      boxShadow: {
        'gy-glow':
          '0 0 0 1px rgba(52,211,153,0.15), 0 0 24px rgba(16,185,129,0.18), 0 0 60px rgba(16,185,129,0.08)',
        'gy-map': '0 0 80px rgba(16,185,129,0.06)',
        'gy-hot-hv': '0 0 40px rgba(16,185,129,0.12)',
        'gy-feed': '0 0 40px rgba(16,185,129,0.05)',
        'gy-eth': '0 0 8px rgba(16,185,129,0.30)',
        'gy-fire': '0 0 8px rgba(251,191,36,0.40)',
      },
      fontFamily: {
        // var(--font-sans) 由 global.css 按 html[data-locale] 切换：zh=Noto Sans SC，en=Inter
        sans: ['var(--font-sans)'],
        // Orbitron 只管拉丁/数字品牌标题
        cyber: ['Orbitron', 'var(--font-sans)'],
        mono: ['JetBrains Mono', 'Consolas', 'Monaco', 'monospace'],
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        glow: 'glow 2s ease-in-out infinite alternate',
        'glow-fast': 'glow 1s ease-in-out infinite alternate',
        float: 'float 6s ease-in-out infinite',
        'spin-slow': 'spin 8s linear infinite',
        fadeInUp: 'fadeInUp 0.5s ease-out forwards',
      },
      keyframes: {
        glow: {
          '0%': { textShadow: '0 0 1px currentColor, 0 0 3px currentColor' },
          '100%': {
            textShadow: '0 0 3px currentColor, 0 0 6px currentColor, 0 0 10px currentColor',
          },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-10px)' },
        },
        fadeInUp: {
          '0%': { opacity: '0', transform: 'translateY(16px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
      },
    },
  },
  plugins: [],
};
