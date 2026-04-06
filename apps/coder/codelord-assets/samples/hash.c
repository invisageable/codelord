/* constant insert/retrieve dictionary for 2-letter words */
#include <stdio.h>
#define LETTERS 26

int hashcode(char *word) {
  return (LETTERS * ((int)word[0]-'a')) + (int)word[1]-'a';
}

void put(char **words, char *word, char *def) {
  words[hashcode(word)] = def;
}

char* get(char **words, char *word) {
  return words[hashcode(word)];
}

int main(int argc, char **argv) {
  char *words[LETTERS*LETTERS];

  put(words, "as", "Like something");
  put(words, "ar", "AR lol");
  put(words, "za", "A funny noise");
  put(words, "qb", "First part of QBC");

  printf("%s\n", get(words, "as"));
  printf("%s\n", get(words, "ar"));
  printf("%s\n", get(words, "za"));
  printf("%s\n", get(words, "qb"));

  return 0;
}
