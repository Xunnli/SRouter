import { useState } from "react"

import { addCustomProvider } from "@/api/providers"
import { toast } from "@/components/arc/toast-stack/toast-stack"

type UseAPIKeyOptions = {
  providerID: string
  connectionName: string
  onConnectionNameChange: (value: string) => void
  onSuccess: () => void
  onOpenChange: (open: boolean) => void
}

export default function useAPIKey({
  providerID,
  connectionName,
  onConnectionNameChange,
  onSuccess,
  onOpenChange,
}: UseAPIKeyOptions) {
  const [apiKey, setAPIKey] = useState("")
  const [prefix, setPrefix] = useState("")
  const [pending, setPending] = useState(false)

  async function save() {
    if (!apiKey.trim()) {
      toast.error("API Key is required")
      return
    }

    setPending(true)
    try {
      // Connect account on an existing provider always adds a new account connection
      // so it never overwrites the previous credentials.
      const label =
        connectionName.trim() ||
        `Account ${new Date().toLocaleDateString("en-US", { month: "numeric", day: "numeric" })}`

      await addCustomProvider({
        name: label,
        api_key: apiKey.trim(),
        prefix: prefix.trim() || undefined,
        protocol: "openai",
        base_url: "inherit",
        ...({ id: providerID } as Record<string, string>),
      })
      toast.success("Account connected successfully!")
      onSuccess()
      onOpenChange(false)
      setAPIKey("")
      onConnectionNameChange("")
      setPrefix("")
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save API key")
    } finally {
      setPending(false)
    }
  }

  function reset() {
    setAPIKey("")
  }

  return {
    apiKey,
    setAPIKey,
    prefix,
    setPrefix,
    pending,
    save,
    reset,
  }
}
