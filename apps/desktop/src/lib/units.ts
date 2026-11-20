/** Unit conversion for the search box: "10 km to mi", "72 f in c", "5 kg para lb". Offline, no money. */

interface Unit {
  /** Names that can be typed, lower case, without dots. */
  names: string[];
  /** How many of the group's base unit one of these is. */
  per: number;
  label: string;
}

const group = (units: Unit[]) => units;

const LENGTH = group([
  { names: ['mm', 'millimeter', 'millimeters', 'milimetro', 'milimetros'], per: 0.001, label: 'mm' },
  { names: ['cm', 'centimeter', 'centimeters', 'centimetro', 'centimetros'], per: 0.01, label: 'cm' },
  { names: ['m', 'meter', 'meters', 'metre', 'metres', 'metro', 'metros'], per: 1, label: 'm' },
  { names: ['km', 'kilometer', 'kilometers', 'kilometre', 'kilometres', 'quilometro', 'quilometros'], per: 1000, label: 'km' },
  { names: ['in', 'inch', 'inches', 'polegada', 'polegadas', 'pulgada', 'pulgadas'], per: 0.0254, label: 'in' },
  { names: ['ft', 'foot', 'feet', 'pe', 'pes', 'pie', 'pies'], per: 0.3048, label: 'ft' },
  { names: ['yd', 'yard', 'yards', 'jarda', 'jardas', 'yarda', 'yardas'], per: 0.9144, label: 'yd' },
  { names: ['mi', 'mile', 'miles', 'milha', 'milhas', 'milla', 'millas'], per: 1609.344, label: 'mi' },
]);
const MASS = group([
  { names: ['mg', 'milligram', 'milligrams', 'miligrama', 'miligramas'], per: 0.000001, label: 'mg' },
  { names: ['g', 'gram', 'grams', 'grama', 'gramas', 'gramo', 'gramos'], per: 0.001, label: 'g' },
  { names: ['kg', 'kilo', 'kilos', 'kilogram', 'kilograms', 'quilo', 'quilos', 'quilograma', 'quilogramas'], per: 1, label: 'kg' },
  { names: ['t', 'ton', 'tons', 'tonne', 'tonnes', 'tonelada', 'toneladas'], per: 1000, label: 't' },
  { names: ['oz', 'ounce', 'ounces', 'onca', 'oncas', 'onza', 'onzas'], per: 0.028349523125, label: 'oz' },
  { names: ['lb', 'lbs', 'pound', 'pounds', 'libra', 'libras'], per: 0.45359237, label: 'lb' },
]);
const VOLUME = group([
  { names: ['ml', 'milliliter', 'milliliters', 'mililitro', 'mililitros'], per: 0.001, label: 'ml' },
  { names: ['l', 'liter', 'liters', 'litre', 'litres', 'litro', 'litros'], per: 1, label: 'l' },
  { names: ['tsp', 'teaspoon', 'teaspoons'], per: 0.00492892159375, label: 'tsp' },
  { names: ['tbsp', 'tablespoon', 'tablespoons'], per: 0.01478676478125, label: 'tbsp' },
  { names: ['cup', 'cups', 'xicara', 'xicaras', 'taza', 'tazas'], per: 0.2365882365, label: 'cup' },
  { names: ['floz', 'fl oz', 'fluidounce', 'fluidounces'], per: 0.0295735295625, label: 'fl oz' },
  { names: ['pt', 'pint', 'pints'], per: 0.473176473, label: 'pt' },
  { names: ['gal', 'gallon', 'gallons', 'galao', 'galoes', 'galon', 'galones'], per: 3.785411784, label: 'gal' },
]);
const SPEED = group([
  { names: ['m/s', 'mps'], per: 1, label: 'm/s' },
  { names: ['km/h', 'kmh', 'kph'], per: 1 / 3.6, label: 'km/h' },
  { names: ['mph', 'mi/h'], per: 0.44704, label: 'mph' },
  { names: ['kn', 'kt', 'knot', 'knots', 'no', 'nos'], per: 0.514444, label: 'kn' },
]);
const DATA = group([
  { names: ['b', 'byte', 'bytes'], per: 1, label: 'B' },
  { names: ['kb'], per: 1000, label: 'KB' },
  { names: ['mb'], per: 1e6, label: 'MB' },
  { names: ['gb'], per: 1e9, label: 'GB' },
  { names: ['tb'], per: 1e12, label: 'TB' },
  { names: ['kib'], per: 1024, label: 'KiB' },
  { names: ['mib'], per: 1024 ** 2, label: 'MiB' },
  { names: ['gib'], per: 1024 ** 3, label: 'GiB' },
  { names: ['tib'], per: 1024 ** 4, label: 'TiB' },
]);
const TIME = group([
  { names: ['s', 'sec', 'secs', 'second', 'seconds', 'segundo', 'segundos'], per: 1, label: 's' },
  { names: ['min', 'mins', 'minute', 'minutes', 'minuto', 'minutos'], per: 60, label: 'min' },
  { names: ['h', 'hr', 'hrs', 'hour', 'hours', 'hora', 'horas'], per: 3600, label: 'h' },
  { names: ['d', 'day', 'days', 'dia', 'dias'], per: 86400, label: 'd' },
  { names: ['wk', 'week', 'weeks', 'semana', 'semanas'], per: 604800, label: 'weeks' },
]);
const AREA = group([
  { names: ['m2', 'sqm'], per: 1, label: 'm²' },
  { names: ['km2', 'sqkm'], per: 1e6, label: 'km²' },
  { names: ['ha', 'hectare', 'hectares', 'hectarea', 'hectareas'], per: 1e4, label: 'ha' },
  { names: ['ft2', 'sqft'], per: 0.09290304, label: 'ft²' },
  { names: ['acre', 'acres'], per: 4046.8564224, label: 'acres' },
]);
const GROUPS = [LENGTH, MASS, VOLUME, SPEED, DATA, TIME, AREA];

const TEMPS: Record<string, 'c' | 'f' | 'k'> = { c: 'c', celsius: 'c', '°c': 'c', f: 'f', fahrenheit: 'f', '°f': 'f', k: 'k', kelvin: 'k' };
const toCelsius = (v: number, from: 'c' | 'f' | 'k') => (from === 'c' ? v : from === 'f' ? ((v - 32) * 5) / 9 : v - 273.15);
const fromCelsius = (c: number, to: 'c' | 'f' | 'k') => (to === 'c' ? c : to === 'f' ? (c * 9) / 5 + 32 : c + 273.15);
const TEMP_LABEL = { c: '°C', f: '°F', k: 'K' } as const;

const norm = (s: string) => s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '').trim();

function findUnit(text: string): { unit: Unit; group: Unit[] } | null {
  const t = norm(text);
  for (const g of GROUPS) {
    const unit = g.find((u) => u.names.includes(t));
    if (unit) return { unit, group: g };
  }
  return null;
}

export interface Converted {
  value: number;
  from: string;
  to: string;
}

/** "10 km to mi" gives { value: 6.21371…, from: 'km', to: 'mi' }. `null` when it isn't a conversion. */
export function convert(query: string): Converted | null {
  const m = /^\s*(-?\d+(?:[.,]\d+)?)\s*([^\d\s][^\s]*(?:\s[^\d\s][^\s]*)?)\s+(?:to|in|into|em|para|a|en|->|=>)\s+(\S+(?:\s\S+)?)\s*$/i.exec(query);
  if (!m) return null;
  const amount = Number(m[1].replace(',', '.'));
  if (!Number.isFinite(amount)) return null;
  const fromText = norm(m[2]);
  const toText = norm(m[3]);
  // Temperatures are not a multiplication, so they go their own way.
  const tf = TEMPS[fromText];
  const tt = TEMPS[toText];
  if (tf && tt) return { value: fromCelsius(toCelsius(amount, tf), tt), from: TEMP_LABEL[tf], to: TEMP_LABEL[tt] };
  const a = findUnit(fromText);
  const b = findUnit(toText);
  if (!a || !b || a.group !== b.group) return null;
  return { value: (amount * a.unit.per) / b.unit.per, from: a.unit.label, to: b.unit.label };
}
