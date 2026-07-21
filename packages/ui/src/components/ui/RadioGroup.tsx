"use client"
import type { ReactNode } from "react"
import {
  Radio as RACRadio,
  RadioGroup as RACRadioGroup,
  composeRenderProps,
} from "react-aria-components"
import type {
  RadioGroupProps as RACRadioGroupProps,
  RadioProps,
  ValidationResult,
} from "react-aria-components"

import {
  Description,
  FieldError,
  Label,
} from "@workspace/ui/components/ui/Field"
import { composeTailwindRenderProps } from "@workspace/ui/lib/react-aria-utils"

export interface RadioGroupProps extends Omit<RACRadioGroupProps, "children"> {
  label?: string
  children?: ReactNode
  description?: string
  errorMessage?: string | ((validation: ValidationResult) => string)
}

export function RadioGroup(props: RadioGroupProps) {
  return (
    <RACRadioGroup
      {...props}
      className={composeTailwindRenderProps(props.className, "s")}
    >
      <Label>{props.label}</Label>
      <div className="flex gap-2 group-orientation-horizontal:gap-4 group-orientation-vertical:flex-col">
        {props.children}
      </div>
      {props.description && <Description>{props.description}</Description>}
      <FieldError>{props.errorMessage}</FieldError>
    </RACRadioGroup>
  )
}

export function Radio(props: RadioProps) {
  return (
    <RACRadio {...props}>
      {composeRenderProps(props.children, (children) => (
        <>{children}</>
      ))}
    </RACRadio>
  )
}
