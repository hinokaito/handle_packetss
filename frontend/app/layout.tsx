import type { Metadata } from "next";
import "./globals.css";
import { ThemeProvider } from "@/components/theme-provider"
import { SidebarProvider, SidebarInset } from "@/components/ui/sidebar"
import { AppSidebar } from "@/components/app-sidebar"
import { AppHeader } from "@/components/app-header"

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
    <html lang="ja" suppressHydrationWarning>
      <head />
      <body className="antialiased">
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem disableTransitionOnChange >
          <SidebarProvider>
            <AppSidebar />
            <SidebarInset>
              <AppHeader />
              <main className="flex-1 overflow-auto p-4">
                {children}
              </main>
            </SidebarInset>
          </SidebarProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
