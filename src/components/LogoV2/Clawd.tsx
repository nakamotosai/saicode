import * as React from 'react'
import { Box, Text } from '../../ink.js'

export type ClawdPose = 'default' | 'arms-up' | 'look-left' | 'look-right'

type Props = {
  pose?: ClawdPose
}

const SAI_LOGO_SEGMENTS = [
  ['██████╗   ', '█████╗  ', '██╗'],
  ['██╔═══╝  ', '██╔══██╗ ', '██║'],
  ['██████╗  ', '███████║ ', '██║'],
  ['╚═══██║  ', '██╔══██║ ', '██║'],
  ['██████║  ', '██║  ██║ ', '██║'],
  ['╚═════╝  ', '╚═╝  ╚═╝ ', '╚═╝'],
] as const

const SAI_STATIC_COLORS = [
  ['rainbow_yellow', 'rainbow_green', 'rainbow_blue'],
  ['rainbow_orange', 'rainbow_blue', 'rainbow_violet'],
  ['rainbow_red', 'rainbow_green', 'rainbow_indigo'],
  ['rainbow_yellow', 'rainbow_blue', 'rainbow_violet'],
  ['rainbow_orange', 'rainbow_green', 'rainbow_blue'],
  ['rainbow_red', 'rainbow_indigo', 'rainbow_violet'],
] as const

export function Clawd(_: Props = {}): React.ReactNode {
  return (
    <Box flexDirection="column">
      {SAI_LOGO_SEGMENTS.map(([sLetter, aLetter, iLetter], index) => (
        <Text key={index}>
          <Text color={SAI_STATIC_COLORS[index]![0]}>{sLetter}</Text>
          <Text color={SAI_STATIC_COLORS[index]![1]}>{aLetter}</Text>
          <Text color={SAI_STATIC_COLORS[index]![2]}>{iLetter}</Text>
        </Text>
      ))}
    </Box>
  )
}
