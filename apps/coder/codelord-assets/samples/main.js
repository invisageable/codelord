export function customTransform(text) {
  return text
    .split(/\b/)
    .map(segment => {
      if (segment.toLowerCase() === 'zo') return 'zo';

      return segment
        .split('')
        .map(char => {
          if (char.toLowerCase() === 'i') return 'i';
          return char.toUpperCase();
        })
        .join('');
    })
    .join('');
}
