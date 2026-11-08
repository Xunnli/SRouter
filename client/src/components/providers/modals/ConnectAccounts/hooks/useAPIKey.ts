import { useState } from "react"

import { addCustomProvider, patchProvider } from "@/api/providers"
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
      // If adding an account / connection with a custom label, register it as a sub-connection
      // through POST /v1/providers with the target provider ID; otherwise update existing row via patch.
      if (connectionName.trim()) {
        await addCustomProvider({
          name: connectionName.trim(),
          api_key: apiKey.trim(),
          prefix: prefix.trim() || undefined,
          protocol: "openai",
          base_url: "inherit",
          ...({ id: providerID } as Record<string, string>),
        })
      } else {
        await patchProvider(providerID, {
          name: connectionName.trim() || undefined,
          api_key: apiKey.trim(),
          prefix: prefix.trim() || undefined,
        })
      }
      toast.success("Provider credentials updated!")
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
