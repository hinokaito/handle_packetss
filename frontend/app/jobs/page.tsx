import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Activity, CreditCard, DollarSign, Users, Server } from "lucide-react"
import { Job } from "@/components/job"

export default function DashboardPage() {
  return (
    <div className="p-4 space-y-4">
      <h2 className="text-3xl font-bold tracking-tight">Jobs</h2>
      <Job />
      <Job />
      <Job />
      <Job />
      <Job />
      <Job />
      <Job />
      <Job />
    </div>
  )
}