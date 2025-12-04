"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"

export function Job() {
    return (
        <Card>
          <CardTitle>Job</CardTitle>
          <CardContent>
            <div>
              <p>Job ID</p>
              <p>Job Name</p>
              <p>Job Description</p>
            </div>
          </CardContent>
        </Card>
    )
}