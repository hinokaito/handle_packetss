"use client"

import { SidebarTrigger } from "@/components/ui/sidebar"
import { ModeToggle } from "@/components/ui/ModeToggle"

export function AppHeader() {
  return (
    <header className="flex h-10 shrink-0 items-center gap-2 border-b px-4">
      <SidebarTrigger />
      <div className="flex flex-1 items-center justify-between">
        <h1 className="text-lg font-semibold">Dashboard</h1>
        <ModeToggle />
      </div>
    </header>
  )
}

