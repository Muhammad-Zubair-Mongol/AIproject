/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        'god': {
          50: '#e6fff2',
          100: '#b3ffe0',
          200: '#80ffce',
          300: '#4dffbc',
          400: '#1affaa',
          500: '#00e68a',  // Primary green
          600: '#00b36b',
          700: '#00804d',
          800: '#004d2e',
          900: '#001a10',
          950: '#000a05',
        },
        'matrix': {
          bg: '#0d0d0d',
          panel: '#1a1a1a',
          border: '#00ff41',
          text: '#00ff41',
          muted: '#008f11',
        }
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'Consolas', 'monospace'],
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'glitch': 'glitch 3s infinite',
        'scan': 'scan 8s linear infinite',
        'flicker': 'flicker 0.15s infinite',
      },
      keyframes: {
        glitch: {
          '0%, 100%': {
            textShadow: '2px 0 rgba(0, 255, 0, 0.7), -2px 0 rgba(0, 0, 255, 0.7)'
          },
          '50%': {
            textShadow: '-2px 0 rgba(0, 255, 0, 0.7), 2px 0 rgba(0, 0, 255, 0.7)'
          },
        },
        scan: {
          '0%': { transform: 'translateY(-100%)' },
          '100%': { transform: 'translateY(100vh)' },
        },
        flicker: {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0.8' },
        },
      },
      backdropBlur: {
        xs: '2px',
      },
      boxShadow: {
        'glow': '0 0 20px rgba(0, 255, 65, 0.3)',
        'glow-lg': '0 0 40px rgba(0, 255, 65, 0.5)',
      },
    },
  },
  plugins: [],
}
