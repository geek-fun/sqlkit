<script setup lang="ts">
import { DatabaseType } from '@/store/connectionStore'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

const props = withDefaults(defineProps<Props>(), {
  databaseType: undefined,
})

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'confirm', name: string, options: { charset?: string, collation?: string, encoding?: string, locale?: string }): void
}>()

const MYSQL_CHARSETS = [
  'utf8mb4',
  'utf8mb3',
  'utf16',
  'utf32',
  'latin1',
  'latin2',
  'ascii',
  'binary',
  'cp1251',
  'cp1257',
  'big5',
  'gbk',
]

const PG_ENCODINGS = ['UTF8', 'LATIN1', 'LATIN2', 'LATIN3', 'LATIN4', 'SQL_ASCII', 'BIG5', 'EUC_JP', 'EUC_KR', 'GB18030', 'GBK', 'ISO_8859_5', 'ISO_8859_13', 'ISO_8859_15', 'KOI8R', 'KOI8U', 'UNICODE', 'WIN1250', 'WIN1251', 'WIN1252', 'WIN866']

 const MYSQL_COMPAT = new Set<DatabaseType>([
   DatabaseType.MYSQL,
   DatabaseType.MARIADB,
   DatabaseType.TIDB,
   DatabaseType.OCEANBASE,
   DatabaseType.TDSQL,
   DatabaseType.POLARDB,
   DatabaseType.DORIS,
   DatabaseType.SELECTDB,
   DatabaseType.STARROCKS,
   DatabaseType.DATABEND,
   DatabaseType.GOLDENDB,
   DatabaseType.MANTICORESEARCH,
   DatabaseType.SINGLESTOREMEMSQL,
   DatabaseType.CLOUDSQLMYSQL,
 ])

 const PG_COMPAT = new Set<DatabaseType>([
   DatabaseType.POSTGRESQL,
   DatabaseType.COCKROACHDB,
   DatabaseType.REDSHIFT,
   DatabaseType.YUGABYTEDB,
   DatabaseType.TIMESCALEDB,
   DatabaseType.KINGBASEES,
   DatabaseType.GAUSSDB,
   DatabaseType.HIGHGO,
   DatabaseType.UXDB,
   DatabaseType.OPENGAUSS,
   DatabaseType.GBASE8C,
   DatabaseType.QUESTDB,
   DatabaseType.VASTBASE,
   DatabaseType.YASHANDB,
   DatabaseType.GREENPLUM,
   DatabaseType.ENTERPRISEDB,
   DatabaseType.CRATEDB,
   DatabaseType.MATERIALIZE,
   DatabaseType.ALLOYDB,
   DatabaseType.CLOUDSQLPG,
   DatabaseType.FUJITSUPG,
 ])

type Props = {
  open: boolean
  databaseType?: DatabaseType
}

const { t } = useI18n()

const objectName = ref('')
const charset = ref('')
const collation = ref('')
const encoding = ref('')
const locale = ref('')

const hasCharset = computed(() => props.databaseType && MYSQL_COMPAT.has(props.databaseType))
const hasCollation = computed(() => props.databaseType && MYSQL_COMPAT.has(props.databaseType))
const hasEncoding = computed(() => props.databaseType && PG_COMPAT.has(props.databaseType))
const hasLocale = computed(() => props.databaseType && PG_COMPAT.has(props.databaseType))

watch(() => props.open, (open) => {
  if (open) {
    objectName.value = ''
    charset.value = 'utf8mb4'
    collation.value = ''
    encoding.value = 'UTF8'
    locale.value = ''
  }
})

function handleConfirm() {
  const trimmed = objectName.value.trim()
  if (!trimmed)
    return
  emit('confirm', trimmed, {
    charset: hasCharset.value ? charset.value || undefined : undefined,
    collation: hasCollation.value ? collation.value || undefined : undefined,
    encoding: hasEncoding.value ? encoding.value || undefined : undefined,
    locale: hasLocale.value ? locale.value || undefined : undefined,
  })
}
</script>

<template>
  <Dialog :open="open" @update:open="(v: boolean) => emit('update:open', v)">
    <DialogContent class="sm:max-w-[420px]">
      <DialogHeader>
        <DialogTitle>{{ t('sidebar.databases.actions.createDatabase.title') }}</DialogTitle>
      </DialogHeader>

      <div class="space-y-3">
        <div class="space-y-1.5">
          <Label class="text-xs">{{ t('sidebar.databases.actions.createDatabase.name') }}</Label>
          <Input
            v-model="objectName"
            :placeholder="t('sidebar.databases.actions.createDatabase.namePlaceholder')"
            class="text-xs"
            @keydown.enter="handleConfirm"
          />
        </div>

        <!-- MySQL options -->
        <template v-if="hasCharset">
          <div class="space-y-1.5">
            <Label class="text-xs">{{ t('sidebar.databases.actions.createDatabase.charset') }}</Label>
            <Select v-model="charset">
              <SelectTrigger class="text-xs h-8">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="cs in MYSQL_CHARSETS" :key="cs" :value="cs" class="text-xs">
                  {{ cs }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label class="text-xs">{{ t('sidebar.databases.actions.createDatabase.collation') }}</Label>
            <Input
              v-model="collation"
              :placeholder="t('sidebar.databases.actions.createDatabase.collationPlaceholder')"
              class="text-xs"
              @keydown.enter="handleConfirm"
            />
          </div>
        </template>

        <!-- PostgreSQL options -->
        <template v-if="hasEncoding">
          <div class="space-y-1.5">
            <Label class="text-xs">{{ t('sidebar.databases.actions.createDatabase.encoding') }}</Label>
            <Select v-model="encoding">
              <SelectTrigger class="text-xs h-8">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="enc in PG_ENCODINGS" :key="enc" :value="enc" class="text-xs">
                  {{ enc }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label class="text-xs">{{ t('sidebar.databases.actions.createDatabase.locale') }}</Label>
            <Input
              v-model="locale"
              :placeholder="t('sidebar.databases.actions.createDatabase.localePlaceholder')"
              class="text-xs"
              @keydown.enter="handleConfirm"
            />
          </div>
        </template>
      </div>

      <DialogFooter>
        <Button variant="outline" size="sm" @click="emit('update:open', false)">
          {{ t('sidebar.databases.actions.createDatabase.cancel') }}
        </Button>
        <Button size="sm" :disabled="!objectName.trim()" @click="handleConfirm">
          {{ t('sidebar.databases.actions.createDatabase.confirm') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
