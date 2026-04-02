import React from 'react'
import { Box, Text } from 'src/ink.js'
import { Clawd } from './Clawd.js'

const WELCOME_V2_WIDTH = 58

function RainbowSaicodeWord() {
  return (
    <>
      <Text color="rainbow_red">s</Text>
      <Text color="rainbow_orange">a</Text>
      <Text color="rainbow_yellow">i</Text>
      <Text color="rainbow_green">c</Text>
      <Text color="rainbow_blue">o</Text>
      <Text color="rainbow_indigo">d</Text>
      <Text color="rainbow_violet">e</Text>
    </>
  )
}

export function WelcomeV2() {
  return (
    <Box width={WELCOME_V2_WIDTH} flexDirection="column">
      <Text>
        <Text>Welcome to </Text>
        <RainbowSaicodeWord />
        <Text> </Text>
        <Text dimColor>v{MACRO.VERSION}</Text>
      </Text>
      <Text dimColor>{'·'.repeat(WELCOME_V2_WIDTH)}</Text>
      <Box marginTop={1} marginBottom={1}>
        <Clawd />
      </Box>
      <Text dimColor>AI coding agent runtime for your local workflow.</Text>
    </Box>
  )
}
