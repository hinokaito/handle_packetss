"use client"
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"

export function KPISummaryCards() {
    return (
        <div>
          
            <Card>
              <CardHeader>Total Throughput</CardHeader>  
              <CardContent>content</CardContent>
            </Card>

            <Card>
              <CardHeader>Global SLA</CardHeader>  
              <CardContent>content</CardContent>
            </Card>

            <Card>
              <CardHeader>Total Efficiency</CardHeader>  
              <CardContent>content</CardContent>
            </Card>

            <Card>
              <CardHeader>Active Regions</CardHeader>  
              <CardContent>content</CardContent>
            </Card>

        </div>
    )
}