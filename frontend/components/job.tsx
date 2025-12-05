"use client"

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { DollarSign, Target, Zap } from "lucide-react"
import Link from "next/link"

export interface JobProps {
  id: string
  title: string
  description: string
  budget: number
  slaTarget: number
  requiredLevel: number
}

function getLevelColor(level: number): string {
  if (level <= 1) return "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
  if (level <= 2) return "bg-sky-500/20 text-sky-400 border-sky-500/30"
  if (level <= 3) return "bg-amber-500/20 text-amber-400 border-amber-500/30"
  if (level <= 4) return "bg-orange-500/20 text-orange-400 border-orange-500/30"
  return "bg-rose-500/20 text-rose-400 border-rose-500/30"
}

function getLevelLabel(level: number): string {
  if (level <= 1) return "Beginner"
  if (level <= 2) return "Easy"
  if (level <= 3) return "Normal"
  if (level <= 4) return "Hard"
  return "Expert"
}

export function Job({ id, title, description, budget, slaTarget, requiredLevel }: JobProps) {
  return (
    <Link href={`/jobs/${id}`}>
      <Card className="group cursor-pointer transition-all duration-300 hover:shadow-lg hover:shadow-primary/10 hover:border-primary/50 bg-card/50 backdrop-blur-sm">
        <CardHeader className="pb-3">
          <div className="flex items-start justify-between gap-2">
            <div className="space-y-1">
              <CardTitle className="text-lg font-bold tracking-tight group-hover:text-primary transition-colors">
                {title}
              </CardTitle>
              <CardDescription className="text-sm text-muted-foreground line-clamp-2">
                {description}
              </CardDescription>
            </div>
            <Badge className={`${getLevelColor(requiredLevel)} shrink-0`}>
              Lv.{requiredLevel} {getLevelLabel(requiredLevel)}
            </Badge>
          </div>
        </CardHeader>
        <CardContent className="pt-0">
          <div className="flex items-center gap-4 text-sm text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <DollarSign className="h-4 w-4 text-emerald-500" />
              <span className="font-mono">${budget}</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Target className="h-4 w-4 text-sky-500" />
              <span className="font-mono">{(slaTarget * 100).toFixed(0)}% SLA</span>
            </div>
            <div className="flex items-center gap-1.5 ml-auto opacity-0 group-hover:opacity-100 transition-opacity">
              <Zap className="h-4 w-4 text-amber-500" />
              <span className="text-xs">Start Mission</span>
            </div>
          </div>
        </CardContent>
      </Card>
    </Link>
  )
}