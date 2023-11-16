#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum Led {
  Led1,
  Led2,
  Led3,
  Led4,
} Led;

typedef enum LedState {
  On,
  Off,
} LedState;

typedef struct Command {
  bool success;
  enum Led led;
  enum LedState state;
} Command;

struct Command parse_uart(const uint8_t *input, uint32_t length);
