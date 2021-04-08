using System.Threading.Tasks;
using Volo.Abp.DependencyInjection;

namespace LogBook.Data
{
    /* This is used if database provider does't define
     * ILogBookDbSchemaMigrator implementation.
     */
    public class NullLogBookDbSchemaMigrator : ILogBookDbSchemaMigrator, ITransientDependency
    {
        public Task MigrateAsync()
        {
            return Task.CompletedTask;
        }
    }
}