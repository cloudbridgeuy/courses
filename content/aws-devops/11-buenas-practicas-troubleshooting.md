+++
title = "Buenas prácticas y troubleshooting"
+++

:::inline-slide light
## Escribir templates que se puedan mantener

Un template funciona o no funciona, pero entre dos templates que funcionan hay una gran
diferencia de calidad: uno se entiende y se modifica con confianza, el otro es un campo
minado. Estas son las prácticas que separan a uno del otro.
:::

:::inline-slide with-title
### Nombres lógicos descriptivos

:::skip
El nombre lógico de un recurso es para las personas que leen el template. `TablaApp` o
`ServicioApp` dicen qué es el recurso; `Resource1` o `MyTable2` no dicen nada. Como el
nombre lógico es además cómo el resto del template se refiere al recurso con `!Ref`, un
buen nombre hace legible cada conexión.
:::

```yaml
MyService2:                    # ¿qué es?
  Type: AWS::ECS::Service
  Properties:
    Cluster: !Ref Resource1    # ¿a qué apunta?

ServicioApp:                   # se lee solo
  Type: AWS::ECS::Service
  Properties:
    Cluster: !Ref ClusterApp
```
:::

:::inline-slide light with-title
### Dejar que AWS genere los nombres físicos

:::skip
Salvo que haya una razón concreta, no se fija el nombre físico de un recurso (`TableName`,
`RoleName`, etc.). Si se deja sin especificar, CloudFormation genera uno único. Esto
evita un problema común: lanzar el mismo template dos veces y que falle porque el nombre
físico ya existe.
:::

```yaml
TablaVisitas:
  Type: AWS::DynamoDB::Table
  Properties:
    TableName: visitas    # el segundo stack falla: "visitas already exists"

# Sin TableName, CloudFormation genera un nombre único:
# taller-aws-maria-TablaVisitas-1AB2C3D4E5F6
```
:::

:::inline-slide with-title
### Parámetros con valores por defecto

:::skip
Un parámetro con `Default` documenta el valor habitual y permite lanzar el stack sin
tener que completarlo cada vez. Se reservan los parámetros sin default para lo que de verdad
cambia entre lanzamientos, como el `ImageUri` del template.
:::

```yaml
Parameters:
  DesiredCount:
    Type: Number
    Default: 1
    Description: Número de tareas en ejecución.
```
:::

:::inline-slide with-title light
### `DeletionPolicy` para lo que no debe perderse

:::skip
Por defecto, borrar un stack borra todos sus recursos. Para los que guardan datos (una
tabla, un bucket) eso puede ser un desastre. `DeletionPolicy` cambia ese
comportamiento, y acepta tres valores:
:::


| Valor | Qué hace al borrar el stack |
| --- | --- |
| `Delete` | Borra el recurso. Es el valor por defecto. |
| `Retain` | Deja el recurso en pie, sin stack que lo gestione. |
| `RetainExceptOnCreate` | Como `Retain`, salvo si se deshace la creación inicial. |
| `Snapshot` | Toma una copia y después borra el recurso. |


```yaml
  TablaClientes:
    Type: AWS::DynamoDB::Table
    DeletionPolicy: Retain
```
:::

`Snapshot` es el punto medio. No deja recursos huérfanos acumulando costo, pero tampoco
pierde los datos. Solo lo admite una lista corta de tipos que saben sacar copias:
`AWS::RDS::DBCluster` y `AWS::RDS::DBInstance`, `AWS::Redshift::Cluster`,
`AWS::Neptune::DBCluster`, `AWS::DocDB::DBCluster`, las dos variantes de
`AWS::ElastiCache`, y `AWS::EC2::Volume`. **DynamoDB no está en esa lista**: para la
tabla del taller la única protección disponible es `Retain`.

:::info
`RetainExceptOnCreate` resuelve una molestia concreta de `Retain`. Si un stack falla al
crearse y hace rollback, `Retain` conserva igual el recurso (vacío, recién nacido, sin
datos que valga la pena salvar) y hay que borrarlo a mano antes de reintentar. Con
`RetainExceptOnCreate`, ese caso se limpia solo, y el resto se sigue conservando.
:::

Note que el template de la Semana 1 **no** marca su `TablaApp` con `Retain`, y es a
propósito: sus datos son descartables, y el ciclo de destrucción y recreación de la
Semana 1 exige que borrar el stack no deje nada atrás. `Retain` es para datos que
duelen perder, no un valor por defecto.

:::inline-slide light with-title
### `UpdateReplacePolicy`: la mitad que suele faltar

:::skip
`DeletionPolicy` protege el recurso cuando **se borra el stack**. No hace nada cuando
el stack sigue vivo y una actualización obliga a **reemplazar** el recurso. Utilizar
`Replacement: True` del change set. En ese caso CloudFormation crea uno nuevo y borra
el viejo, con `Retain` puesto y todo.
:::

:::skip
No es un descuido de nadie: es el comportamiento documentado. `DeletionPolicy` cubre
que el recurso se borre del stack, y **no cubre** que su instancia física se reemplace
durante una actualización. Ese hueco lo cierra `UpdateReplacePolicy`, que acepta los
mismos valores y se aplica al reemplazo. Por eso el template de datos lleva los dos:
:::

:::add visibility=slide
`DeletionPolicy` cubre que el recurso se borre del stack, y **no cubre** que su
instancia física se reemplace durante una actualización. Ese hueco lo cierra
`UpdateReplacePolicy`, que acepta los mismos valores y se aplica al reemplazo.
:::


```yaml
  TablaApp:
    Type: AWS::DynamoDB::Table
    DeletionPolicy: Retain          # al borrar el stack
    UpdateReplacePolicy: Retain     # al reemplazar la tabla en una actualización
```

:::skip
La regla es simple: **los dos atributos van juntos, siempre**. Poner solo
`DeletionPolicy` da una sensación de seguridad que no se corresponde con la realidad,
y el día que alguien cambie una propiedad que exige reemplazo, los datos se van sin
que nadie haya borrado nada.
:::

:::add
::: info
**Los dos atributos van juntos, siempre**. Poner solo
`DeletionPolicy` da una sensación de seguridad que no se corresponde con la realidad,
y el día que alguien cambie una propiedad que exige reemplazo, los datos se van sin
que nadie haya borrado nada.
::: #info
::: #add
::: #inline-slide

:::inline-slide light with-title
### Etiquetar el stack, no cada recurso

:::skip
Las etiquetas de la sección anterior se ponían recurso por recurso. Al lanzar o
actualizar un stack, la pantalla **Configure stack options** ofrece un juego de
etiquetas a nivel de stack, y CloudFormation las **propaga a todos los recursos que
las soporten**. Un solo lugar en vez de decenas.

Sirven para tres cosas concretas: filtrar los recursos del taller en la consola,
repartir el costo en Cost Explorer, y encontrar lo que quedó vivo al limpiar la
cuenta. Un mínimo razonable:
:::


| Etiqueta | Valor de ejemplo |
| --- | --- |
| `Proyecto` | `taller-aws-devops` |
| `Ambiente` | `taller` |
| `Responsable` | `{%nombre%}` |


::: info
Para que una etiqueta aparezca en los informes de costo hay que activarla como
*cost allocation tag* en **Billing → Cost allocation tags**, una vez por cuenta. Hasta
entonces la etiqueta existe en el recurso, pero Cost Explorer no puede agrupar por
ella.
:::

::: warning
Nunca modificar a mano, desde la consola, un recurso que gestiona un stack. El template
y la realidad dejan de coincidir (*drift*) y la próxima actualización del stack
puede revertir el cambio sin avisar. Si un recurso lo gestiona un stack, cambiarlo solo a
través del stack.
::: # warning
::: # inline-slide

:::inline-slide with-title
### Los outputs son un contrato

La sección `Outputs` es la interfaz pública del stack: lo que expone para que
otros lo usen. La URL del ALB es un output porque alguien (otro stack, un
script, una persona) necesita ese valor sin tener que entrar a buscarlo. Los
outputs son un contrato: se nombran con claridad y se expone lo que realmente
se consume desde afuera.
:::

## Buenas prácticas, en una línea

- Nombres lógicos **descriptivos**.
- Dejar que AWS **genere los nombres físicos**.
- Parámetros con **`Default`** para lo habitual.
- **`DeletionPolicy`** y **`UpdateReplacePolicy`** en `Retain` para los datos.
- Etiquetas **a nivel de stack**, no recurso por recurso.
- Nunca editar a mano un recurso gestionado por un stack.

:::inline-slide
## Proteger un stack de un borrado accidental

Las políticas de la sección anterior protegen **un recurso**. Hay dos mecanismos más
que protegen **el stack entero**, y los dos se configuran fuera del template: viven en
el stack, no en el archivo.

1. Termination Protection
2. Stack Policies
:::

:::inline-slide
### Termination protection

:::skip
Con la protección activada, la acción **Delete** falla con un mensaje claro en lugar de
borrar nada. Hay que desactivarla a propósito, en una operación aparte, antes de poder
borrar el stack. Ese segundo paso deliberado es toda la protección: convierte un clic
distraído en una decisión.

Se activa en **Stack actions → Edit termination protection**, y es la primera cosa que
conviene hacer con un stack que guarda datos —el de datos de la sección anterior es el
candidato obvio—. El stack de aplicación, en cambio, es descartable a propósito, y
protegerlo solo estorbaría.
:::

:::add visibility=slide
La acción **Delete** falla con un mensaje claro a menos que se desactive la función.

**Stack actions → Edit termination protection**
:::

::: info
La protección es **por stack**: en un stack anidado se configura en el stack raíz, y
los hijos la heredan.
::: #info
::: #inline-slide

:::inline-slide
### Stack policies

:::skip
La termination protection cubre el borrado del stack completo. No dice nada sobre una
actualización que reemplace un recurso concreto. Para eso está la **stack policy**: un
documento JSON, asociado al stack, que declara qué recursos puede tocar una
actualización y cuáles no.
:::

:::add visibility=slide
La **stack policy** es un documento JSON asociado al stack, que declara que recursos
puede tocar una actualización y cuáles no.
:::

```json
{
  "Statement": [
    { "Effect": "Allow", "Action": "Update:*", "Principal": "*", "Resource": "*" },
    { "Effect": "Deny",  "Action": ["Update:Replace", "Update:Delete"],
      "Principal": "*", "Resource": "LogicalResourceId/TablaApp" }
  ]
}
```

:::skip
Se lee de arriba abajo: todo permitido, salvo reemplazar o borrar `TablaApp`. Un change
set que intente cualquiera de las dos cosas se rechaza al ejecutarse.

Conviene entender las diferencias con las otras dos herramientas, porque las tres se
confunden:
::: # skip
::: # inline-slide

:::inline-slide
:::add visibility=slide
## Tres protecciones, tres alcances
:::


| Mecanismo | Qué bloquea | Dónde vive |
| --- | --- | --- |
| `DeletionPolicy` / `UpdateReplacePolicy` | Que el recurso **se pierda** | En el template |
| Stack policy | Que una actualización **toque** ciertos recursos | En el stack |
| Termination protection | Que se **borre el stack** | En el stack |

:::skip
Las dos primeras se complementan: la política del template salva los datos, la stack
policy impide que la operación llegue a ocurrir. Y la stack policy tiene una virtud que
`UpdateReplacePolicy` no tiene: falla **antes**, con un error, en vez de dejar un
recurso huérfano.
:::

::: warning
Una stack policy no protege contra IAM ni reemplaza a IAM. Solo limita lo que las
actualizaciones de **ese stack** pueden hacer. Quien tenga permisos suficientes puede
cambiar la política, o borrar el recurso desde la consola del servicio, por fuera del
stack. Es una red de seguridad contra el error propio, no un control de acceso.
::: #warning
::: #inline-slide

:::inline-slide
## Troubleshooting: leer un fallo

Tarde o temprano un stack falla. Saber leer el fallo es lo que convierte media hora de
frustración en dos minutos de diagnóstico.
:::

:::inline-slide with-title light
### El primer evento fallido es la causa

Cuando un stack entra en `ROLLBACK_IN_PROGRESS`, la pestaña **Events** se llena de
mensajes: el recurso que falló, y luego todos los que se deshacen en el rollback. El
ruido del rollback puede esconder la causa real.

La técnica: ordene los eventos por tiempo y busque el **primer** evento con
estado `CREATE_FAILED` (o `UPDATE_FAILED`). Ese, y no los posteriores, contiene
el motivo real, el resto son consecuencias. La columna de razón (*status
reason*) suele decir exactamente qué pasó.
:::

:::inline-slide with-title
### Fallos comunes y qué los causa

| Síntoma | Causa habitual |
| --- | --- |
| `requires capabilities: [CAPABILITY_IAM]` | El template crea roles de IAM; falta marcar la casilla de capacidades al lanzar. |
| `already exists` | Colisión de nombre físico: el recurso ya existe (a menudo, por fijar un nombre a mano). |
| `is not authorized to perform` | Permisos insuficientes en el usuario o rol que lanza el stack. |
| `limit exceeded` | Se alcanzó un límite de la cuenta (por ejemplo, número de VPC o de EIP). |

:::
La primera fila explica la casilla de capacidades que se marcó en la Semana 1: no era
un trámite, era CloudFormation pidiendo permiso explícito para crear roles de IAM.

:::inline-slide light with-title
### Los límites del servicio

Un último grupo de fallos no viene del template sino de los **límites de
CloudFormation**. Aparecen tarde, cuando un stack ya creció, y desconciertan porque el
template es válido:

| Límite | Valor |
| --- | --- |
| Recursos por stack | 500 |
| Parámetros, mappings, u outputs por template | 200 cada uno |
| Tamaño del template subido desde el navegador | 51.200 bytes |
| Tamaño del template alojado en S3 | 1 MB |
:::add visibility=slide
El límite de 51.200 bytes es el que se encuentra primero: pasado ese tamaño, la consola
deja de aceptar **Upload a template file** y hay que subir el archivo a un bucket de S3
y darle la URL.
::: # add
::: # inline-slide

El límite de 51.200 bytes es el que se encuentra primero: pasado ese tamaño, la consola
deja de aceptar **Upload a template file** y hay que subir el archivo a un bucket de S3
y darle la URL. Los límites de recursos y de outputs se alcanzan mucho después, y
cuando pasa, la respuesta no es pelearlos: es **dividir el stack** —exactamente lo que
se hizo en la sección anterior, ahora por una razón distinta—.

::: extra Validar el template antes de lanzarlo
La herramienta de línea de comandos **`cfn-lint`** revisa un template en busca de errores
de sintaxis, propiedades inválidas, y referencias rotas, sin necesidad de lanzar nada.
Integrada en el editor o en el pipeline, atrapa la mayoría de los errores antes de llegar
a la consola. La consola también ofrece un botón **Validate** que hace una verificación
básica de sintaxis al subir el template.
:::

::: extra Lo mismo desde la línea de comandos
Todo lo que se hizo por la consola tiene su equivalente en el CLI, y un pipeline
automatizado lo necesita —la consola no se puede scriptear—. El comando central es
`aws cloudformation deploy`, que crea el stack si no existe y lo actualiza si existe,
usando un change set por debajo:

```bash
aws cloudformation deploy \
  --stack-name taller-aws-{%nombre%}-app \
  --template-file taller-aws-devops-semana2-app.yaml \
  --parameter-overrides ImageUri=... RedStackName=... DatosStackName=... PlataformaStackName=... \
  --capabilities CAPABILITY_IAM
```

Con `--no-execute-changeset`, el comando se detiene después de calcular el change set,
sin aplicarlo: es la versión automatizable del "revisar antes de ejecutar" de la
sesión anterior, y lo que permite meter una aprobación humana en el medio.

Otros comandos útiles: `validate-template` (sintaxis), `describe-stack-events`
(la pestaña **Events**), `describe-stacks --query "Stacks[0].Outputs"` (los outputs, en
un script), y `wait stack-create-complete` (bloquea hasta que el stack termine).
La Semana 3 los retoma al armar el pipeline.
:::
