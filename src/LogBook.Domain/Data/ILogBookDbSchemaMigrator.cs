using System.Threading.Tasks;

namespace LogBook.Data
{
    public interface ILogBookDbSchemaMigrator
    {
        Task MigrateAsync();
    }
}
