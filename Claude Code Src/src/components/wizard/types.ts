export type WizardStepComponent<T = any> = (props: {
  data: T
  setData: (updater: T | ((prev: T) => T)) => void
  next: () => void
  back: () => void
  cancel: () => void
  [key: string]: any
}) => React.ReactNode

export type WizardProviderProps<T = any> = {
  children?: React.ReactNode
  initialData?: T
  steps?: WizardStepComponent<T>[]
  onDone?: (data: T) => void
  onCancel?: () => void
  [key: string]: any
}

export type WizardContextValue<T = any> = {
  data: T
  setData: (updater: T | ((prev: T) => T)) => void
  currentStep: number
  next: () => void
  back: () => void
  cancel: () => void
  [key: string]: any
}
