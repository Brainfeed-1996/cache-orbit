/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
        sans: ['Inter', 'system-ui', 'monospace'],
      },
      colors: {
        orbit: {
          950: '#020617',
          900: '#0f172a',
          800: '#1e293b',
          accent: '#38bdf8',
          warn: '#fbbf24',
          danger: '#ef4444',
          success: '#22c55e',
        },
      },
    },
  },
  plugins: [],
};
