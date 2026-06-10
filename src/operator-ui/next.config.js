/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  experimental: {
    serverActions: true,
  },
  images: {
    remotePatterns: [],
  },
};

module.exports = nextConfig;
