export type SubscriptionType =
  | 'free'
  | 'pro'
  | 'max'
  | 'team'
  | 'enterprise'

export type RateLimitTier = string
export type BillingType = string

export type OAuthTokenExchangeResponse = {
  access_token: string
  refresh_token?: string
  token_type?: string
  expires_in?: number
  scope?: string
  id_token?: string
  account?: {
    uuid?: string
    email_address?: string
    email?: string
    [key: string]: unknown
  }
  organization?: {
    uuid?: string
    organization_type?: string
    rate_limit_tier?: string
    [key: string]: unknown
  }
}

export type OAuthProfileResponse = Record<string, unknown> & {
  account: {
    uuid?: string
    email?: string
    display_name?: string
    created_at?: string
    [key: string]: unknown
  }
  organization: {
    uuid?: string
    has_extra_usage_enabled?: boolean
    billing_type?: string
    subscription_created_at?: string
    [key: string]: unknown
  }
}

export type UserRolesResponse = {
  organization_role?: string
  workspace_role?: string
  organization_name?: string
  [key: string]: unknown
}

export type OAuthTokens = {
  accessToken: string
  refreshToken?: string
  expiresAt?: number
  scopes?: string[]
  profile?: OAuthProfileResponse | null
  tokenAccount?: {
    uuid?: string
    emailAddress?: string
    organizationUuid?: string
    organizationType?: string
    rateLimitTier?: string
    [key: string]: unknown
  } | null
  subscriptionType?: SubscriptionType | null
  rateLimitTier?: RateLimitTier | null
  billingType?: BillingType | null
  account?: Record<string, unknown> | null
}
