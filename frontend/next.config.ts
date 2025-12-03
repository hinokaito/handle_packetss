import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Turbopack configuration (default in Next.js 16)
  turbopack: {
    // WebAssembly support is built-in with Turbopack
  },
  // Keep webpack config for compatibility during migration
  webpack: (config) => {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
    };
    return config;
  },
};

export default nextConfig;
