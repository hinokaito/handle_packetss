import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Activity, CreditCard, DollarSign, Users, Server } from "lucide-react"

export default function DashboardPage() {
  return (
    <div className="p-4 space-y-4">
      <h2 className="text-3xl font-bold tracking-tight">Dashboard</h2>
      
      {/* Main Content */}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4 auto-rows-[180px]">
        
        {/* KPI Summary Cards */}
        <Card className="col-span-1 md:col-span-2 lg:col-span-2">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Throughput</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">45.2k req/s</div>
            <p className="text-xs text-muted-foreground">+20.1% from last hour</p>
            {/* Graph here... */}
            <div className="h-[80px] w-full bg-zinc-800/50 mt-4 rounded animate-pulse" />
          </CardContent>
        </Card>

        {/* Global SLA */}
        <Card className="col-span-1">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Global SLA</CardTitle>
            <Server className="h-4 w-4 text-green-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-500">99.99%</div>
            <p className="text-xs text-muted-foreground">All systems operational</p>
          </CardContent>
        </Card>

        {/* Revenue */}
        <Card className="col-span-1">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Revenue</CardTitle>
            <DollarSign className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">$12,234</div>
            <p className="text-xs text-muted-foreground">+$234 this hour</p>
          </CardContent>
        </Card>

        {/* Recent Alerts */}
        <Card className="col-span-1 row-span-2">
          <CardHeader>
            <CardTitle>Recent Alerts</CardTitle>
            <CardDescription>Latest system events</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4 text-sm">
              <div className="flex items-center">
                <span className="w-2 h-2 bg-red-500 rounded-full mr-2" />
                Stage 1: High Latency
              </div>
              <div className="flex items-center">
                <span className="w-2 h-2 bg-yellow-500 rounded-full mr-2" />
                Stage 2: Budget 80%
              </div>
              <div className="flex items-center">
                <span className="w-2 h-2 bg-green-500 rounded-full mr-2" />
                Stage 3: Deployed
              </div>
            </div>
          </CardContent>
        </Card>

        { /* Activa Regions */ }
        <Card className="col-span-1 md:col-span-1 lg:col-span-3">
          <CardHeader>
            <CardTitle>Active Regions</CardTitle>
            <CardDescription>Currently running simulations</CardDescription>
          </CardHeader>
          <CardContent>
             {/* コンテンツ */}
             <div className="text-muted-foreground">Map or List here...</div>
          </CardContent>
        </Card>

      </div>
    </div>
  )
}