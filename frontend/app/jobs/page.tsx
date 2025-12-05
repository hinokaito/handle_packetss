"use client"

import { useEffect, useState } from "react"
import { Job, JobProps } from "@/components/job"
import { Briefcase, Loader2 } from "lucide-react"

interface StageListItem {
  id: string
  title: string
  description: string
  budget: number
  sla_target: number
  required_level: number
}

export default function JobsPage() {
  const [jobs, setJobs] = useState<StageListItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    async function fetchJobs() {
      try {
        const res = await fetch("http://localhost:8080/api/stages")
        if (!res.ok) {
          throw new Error("Failed to fetch jobs")
        }
        const data: StageListItem[] = await res.json()
        setJobs(data)
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error")
      } finally {
        setLoading(false)
      }
    }
    fetchJobs()
  }, [])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-[50vh]">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="p-4">
        <div className="text-destructive bg-destructive/10 border border-destructive/20 rounded-lg p-4">
          Error: {error}
        </div>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center gap-3">
        <Briefcase className="h-8 w-8 text-primary" />
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Jobs</h2>
          <p className="text-muted-foreground">ミッションを選択してください</p>
        </div>
      </div>
      
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {jobs.map((job) => (
          <Job
            key={job.id}
            id={job.id}
            title={job.title}
            description={job.description}
            budget={job.budget}
            slaTarget={job.sla_target}
            requiredLevel={job.required_level}
          />
        ))}
      </div>

      {jobs.length === 0 && (
        <div className="text-center py-12 text-muted-foreground">
          利用可能なミッションがありません
        </div>
      )}
    </div>
  )
}