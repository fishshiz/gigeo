"use client"

import {
  TextField as AriaTextField,
  type TextFieldProps as AriaTextFieldProps,
  type ValidationResult,
} from "react-aria-components"
import {
  Description,
  FieldError,
  Input,
  Label,
} from "@workspace/ui/components/ui/Field"
import type { InputProps as AriaInputProps } from "react-aria-components"
import { forwardRef } from "react"
import { composeTailwindRenderProps } from "@workspace/ui/lib/react-aria-utils"

export interface TextFieldProps extends AriaTextFieldProps {
  label?: string
  description?: string
  placeholder?: string
  errorMessage?: string | ((validation: ValidationResult) => string)
  inputClassName?: AriaInputProps["className"]
}

export const TextField = forwardRef<HTMLInputElement, TextFieldProps>(
  (
    { label, description, errorMessage, inputClassName, placeholder, ...props },
    ref
  ) => {
    return (
      <AriaTextField
        {...props}
        className={composeTailwindRenderProps(
          props.className,
          "flex flex-col gap-1 font-sans"
        )}
      >
        {label ? <Label>{label}</Label> : null}
        <Input ref={ref} className={inputClassName} placeholder={placeholder} />
        {description ? <Description>{description}</Description> : null}
        <FieldError>{errorMessage}</FieldError>
      </AriaTextField>
    )
  }
)
