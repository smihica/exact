@file:Suppress("UNUSED_PARAMETER")

package androidx.work

import android.content.Context
import java.util.concurrent.TimeUnit

/** Records scheduling requests; tests deliver their wake through BackgroundModule.ran. */
class WorkManager {
    companion object {
        val instance = WorkManager()
        fun getInstance(context: Context) = instance
    }
    val requests = HashMap<String, Request>()
    fun enqueueUniqueWork(name: String, policy: ExistingWorkPolicy, request: Request) { requests[name] = request }
    fun enqueueUniquePeriodicWork(name: String, policy: ExistingPeriodicWorkPolicy, request: Request) { requests[name] = request }
    fun cancelUniqueWork(name: String) { requests.remove(name) }
}

enum class ExistingWorkPolicy { REPLACE }
enum class ExistingPeriodicWorkPolicy { UPDATE }
enum class NetworkType { CONNECTED }

class Constraints {
    class Builder {
        fun setRequiredNetworkType(type: NetworkType) = this
        fun build() = Constraints()
    }
}

class Data(private val values: Map<String, String> = emptyMap()) {
    fun getString(key: String) = values[key]
    class Builder {
        private val values = HashMap<String, String>()
        fun putString(key: String, value: String) = apply { values[key] = value }
        fun build() = Data(values.toMap())
    }
}

data class Request(val delay: Long, val data: Data)
open class RequestBuilder {
    private var delay = 0L
    private var data = Data()
    fun setConstraints(constraints: Constraints) = this
    fun setInputData(data: Data) = apply { this.data = data }
    fun setInitialDelay(delay: Long, unit: TimeUnit) = apply { this.delay = unit.toMillis(delay) }
    fun build() = Request(delay, data)
}
class OneTimeWorkRequestBuilder<T : Worker> : RequestBuilder()
class PeriodicWorkRequestBuilder<T : Worker>(interval: Long, unit: TimeUnit, flex: Long, flexUnit: TimeUnit) : RequestBuilder()

class WorkerParameters(val inputData: Data = Data())
abstract class Worker(context: Context, params: WorkerParameters) {
    val inputData = params.inputData
    abstract fun doWork(): Result
    enum class Result {
        FAILURE, RETRY, SUCCESS;
        companion object {
            fun failure() = FAILURE
            fun retry() = RETRY
            fun success() = SUCCESS
        }
    }
}
