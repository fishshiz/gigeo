import { Slider } from "@workspace/ui/components/ui/Slider"
import { useState } from "react"
const DateSlider = ({
  dates,
  handleChange,
}: {
  dates: string[]
  handleChange: (date: string) => void
}) => {
  const [value, setValue] = useState(0)
  const emitChange = (value: number) => {
    handleChange(dates[value])
  }
  return (
    <Slider
      className="w-full"
      label={dates[value]}
      value={value}
      onChange={setValue}
      minValue={0}
      maxValue={dates.length - 1}
      step={1}
      onChangeEnd={emitChange}
    />
  )
}
export { DateSlider }
