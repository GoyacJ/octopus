export interface ApprovalRequestRecord {
  id: string
  sessionId: string
  conversationId: string
  runId: string
  toolName: string
  summary: string
  detail: string
  riskLevel: 'low' | 'medium' | 'high'
  createdAt: number
  status: 'pending' | 'approved' | 'rejected'
}
