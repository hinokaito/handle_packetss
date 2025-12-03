import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "WebSocket + Wasm Demo",
  description: "Real-time packet visualization with WebGPU and Rust/Wasm",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="ja">
      <body className="antialiased">
        {children}
      </body>
    </html>
  );
}
